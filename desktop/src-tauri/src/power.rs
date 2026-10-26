//! Idle-sleep assertions are owned by the import worker and released on every exit.
//! The display may sleep; manual sleep and lid-close behavior remain OS-controlled.
use anyhow::{Result, bail};

#[cfg(target_os = "macos")]
pub struct SleepGuard(u32);
#[cfg(windows)]
pub struct SleepGuard {
    // Windows execution state is thread-local. Do not allow this guard to move.
    _thread_bound: std::marker::PhantomData<std::rc::Rc<()>>,
}
#[cfg(not(any(target_os = "macos", windows)))]
pub struct SleepGuard;

#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        kind: core_foundation::string::CFStringRef,
        level: u32,
        name: core_foundation::string::CFStringRef,
        id: *mut u32,
    ) -> i32;
    fn IOPMAssertionRelease(id: u32) -> i32;
}

impl SleepGuard {
    pub fn acquire(reason: &str) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            use core_foundation::{base::TCFType, string::CFString};
            let kind = CFString::new("PreventUserIdleSystemSleep");
            let name = CFString::new(reason);
            let mut id = 0;
            // Both CFString references remain alive throughout this call.
            let result = unsafe {
                IOPMAssertionCreateWithName(
                    kind.as_concrete_TypeRef(),
                    255,
                    name.as_concrete_TypeRef(),
                    &mut id,
                )
            };
            if result != 0 {
                bail!("IOPMAssertionCreateWithName: {result}");
            }
            Ok(Self(id))
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Power::{
                ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
            };
            let _ = reason;
            if unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) } == 0 {
                bail!("SetThreadExecutionState failed");
            }
            Ok(Self {
                _thread_bound: std::marker::PhantomData,
            })
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            let _ = reason;
            bail!("Sleep prevention is unavailable on this platform");
        }
    }
}
impl Drop for SleepGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe {
            IOPMAssertionRelease(self.0);
        }
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Power::{ES_CONTINUOUS, SetThreadExecutionState};
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    fn assertions() -> String {
        let output = std::process::Command::new("/usr/bin/pmset")
            .args(["-g", "assertions"])
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
    #[test]
    fn assertion_is_held_during_work_and_released_after_success_or_error() {
        let reason = "Framemoor power lifecycle test";
        {
            let _guard = SleepGuard::acquire(reason).unwrap();
            assert!(assertions().contains(reason));
        }
        assert!(!assertions().contains(reason));
        let failed = (|| -> Result<()> {
            let _guard = SleepGuard::acquire(reason)?;
            bail!("simulated import failure");
        })();
        assert!(failed.is_err());
        assert!(!assertions().contains(reason));
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use windows_sys::Win32::System::Power::{
        ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };

    #[test]
    fn execution_state_is_held_and_released_on_the_worker_thread() {
        // This dedicated test thread owns its execution-state flags.
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
        {
            let _guard = SleepGuard::acquire("Framemoor native test").unwrap();
            let previous = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
            assert_ne!(previous & ES_SYSTEM_REQUIRED, 0);
        }
        let previous = unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
        assert_eq!(previous & ES_SYSTEM_REQUIRED, 0);
    }
}
