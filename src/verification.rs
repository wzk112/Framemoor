//! Bounded, read-only verification workers. All journal writes stay on the caller.
use crate::{Control, Paused};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

pub const MAX_READERS: usize = 8;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    /// None chooses a conservative hardware-based value for each run.
    pub readers: Option<usize>,
}
pub fn enabled_by_default() -> bool {
    true
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            readers: None,
        }
    }
}
impl Settings {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.readers.is_none_or(|n| (1..=MAX_READERS).contains(&n)),
            "invalid_verification_readers"
        );
        Ok(())
    }
    pub fn resolve(self, source: &Path, destination: &Path) -> Result<usize> {
        self.validate()?;
        Ok(self
            .readers
            .unwrap_or_else(|| recommendation(source, destination).readers))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Recommendation {
    pub logical_cpus: usize,
    pub memory_bytes: Option<u64>,
    pub same_volume: Option<bool>,
    pub readers: usize,
}

fn choose(cpus: usize, memory: Option<u64>, same_volume: Option<bool>) -> usize {
    // Storage bandwidth is not inferred from CPU count. Unknown media starts at
    // two readers; same-volume and small systems use one. Manual mode is capped.
    if cpus < 4 || memory.is_some_and(|n| n < 4 * 1024 * 1024 * 1024) || same_volume != Some(false)
    {
        1
    } else {
        2
    }
}
pub fn recommendation(source: &Path, destination: &Path) -> Recommendation {
    let logical_cpus = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let memory_bytes = crate::platform::physical_memory();
    let same_volume = crate::platform::mount_token(source)
        .ok()
        .zip(crate::platform::mount_token(destination).ok())
        .map(|(a, b)| a == b);
    Recommendation {
        logical_cpus,
        memory_bytes,
        same_volume,
        readers: choose(logical_cpus, memory_bytes, same_volume),
    }
}

enum Message<T> {
    Started(usize),
    Finished(usize, Result<T>),
}

/// Results retain input order, callbacks run only on the caller, and every
/// worker is joined before returning on success, pause, error, or panic.
pub(crate) fn map<T: Send>(
    count: usize,
    readers: usize,
    job: impl Fn(usize, &AtomicBool) -> Result<T> + Sync,
    mut progress: impl FnMut(usize, usize) -> Result<Control>,
) -> Result<Vec<T>> {
    ensure!(
        (1..=MAX_READERS).contains(&readers),
        "invalid_verification_readers"
    );
    if count == 0 {
        return Ok(Vec::new());
    }
    if matches!(progress(0, 0)?, Control::Pause) {
        return Err(Paused.into());
    }
    let stop = AtomicBool::new(false);
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::new();
        for _ in 0..readers.min(count) {
            let (tx, job, stop, next) = (tx.clone(), &job, &stop, &next);
            handles.push(scope.spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= count {
                        break;
                    }
                    if tx.send(Message::Started(i)).is_err() {
                        break;
                    }
                    let value = job(i, stop);
                    let failed = value.is_err();
                    if tx.send(Message::Finished(i, value)).is_err() {
                        break;
                    }
                    if failed {
                        stop.store(true, Ordering::Release);
                    }
                }
            }));
        }
        drop(tx);
        let mut results: Vec<Option<T>> = (0..count).map(|_| None).collect();
        let (mut current, mut done, mut error) = (0, 0, None);
        loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Message::Started(i)) => current = i,
                Ok(Message::Finished(i, Ok(value))) => {
                    results[i] = Some(value);
                    done += 1;
                    current = i;
                }
                Ok(Message::Finished(_, Err(e))) => {
                    if error.is_none() {
                        error = Some(e);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if error.is_none() {
                match progress(current, done) {
                    Ok(Control::Continue) => {}
                    Ok(Control::Pause) => error = Some(Paused.into()),
                    Err(e) => error = Some(e),
                }
            }
            if error.is_some() {
                stop.store(true, Ordering::Release);
            }
        }
        for handle in handles {
            if handle.join().is_err() && error.is_none() {
                error = Some(anyhow!("Verification worker failed"));
            }
        }
        if let Some(error) = error {
            return Err(error);
        }
        results
            .into_iter()
            .map(|v| v.ok_or_else(|| anyhow!("Verification did not finish")))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_limits_and_invalid_settings() {
        assert_eq!(choose(18, Some(48 << 30), Some(false)), 2);
        assert_eq!(choose(18, Some(48 << 30), Some(true)), 1);
        assert_eq!(choose(2, None, Some(false)), 1);
        assert_eq!(choose(8, Some(2 << 30), Some(false)), 1);
        assert_eq!(choose(8, None, None), 1);
        for readers in [0, 9] {
            assert!(
                Settings {
                    readers: Some(readers),
                    enabled: true
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            serde_json::from_str::<Settings>("{}")
                .unwrap()
                .readers
                .is_none()
        );
    }
    #[test]
    fn bounded_parallel_results_and_drained_workers() -> Result<()> {
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let values = map(
            24,
            4,
            |i, _| {
                let n = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(n, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(5));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(i)
            },
            |_, _| Ok(Control::Continue),
        )?;
        assert_eq!(values, (0..24).collect::<Vec<_>>());
        assert!((2..=4).contains(&peak.load(Ordering::SeqCst)));
        assert_eq!(active.load(Ordering::SeqCst), 0);
        let mut polls = 0;
        assert!(
            map(
                20,
                4,
                |_, stop| {
                    active.fetch_add(1, Ordering::SeqCst);
                    while !stop.load(Ordering::Acquire) {
                        std::thread::yield_now();
                    }
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                },
                |_, _| {
                    polls += 1;
                    Ok(if polls > 2 {
                        Control::Pause
                    } else {
                        Control::Continue
                    })
                }
            )
            .is_err()
        );
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert!(
            map::<()>(
                8,
                4,
                |i, _| {
                    if i == 0 {
                        anyhow::bail!("read error");
                    }
                    Ok(())
                },
                |_, _| Ok(Control::Continue)
            )
            .is_err()
        );
        Ok(())
    }
}
