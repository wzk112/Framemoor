//! Own one metadata command and its helpers until all output has been drained.
use anyhow::{Context, Result};
use std::process::{Child, Command};

pub(super) struct MetadataProcess {
    pub child: Child,
    #[cfg(windows)]
    job: windows_job::Job,
}

impl MetadataProcess {
    pub fn spawn(command: &mut Command) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        let job = {
            use std::os::windows::process::CommandExt;
            use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
            let job = windows_job::Job::new()?;
            // Register the suspended launcher before it can create any helper.
            command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
            job
        };
        let child = command.spawn().context("metadata_tool_failed")?;
        #[cfg(windows)]
        let mut child = child;
        #[cfg(windows)]
        if let Err(error) = job.attach_and_resume(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(Self {
            child,
            #[cfg(windows)]
            job,
        })
    }

    pub fn stop(&mut self) {
        #[cfg(unix)]
        // The process group remains ours even if its launcher has exited while
        // a helper still holds stdout/stderr. Never target a caller's group.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        #[cfg(windows)]
        self.job.terminate();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(windows)]
mod windows_job {
    use anyhow::{Context, Result, ensure};
    use std::{
        io,
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::Child,
    };
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject, TerminateJobObject,
            },
            Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
        },
    };

    pub(super) struct Job(OwnedHandle);

    impl Job {
        pub fn new() -> Result<Self> {
            let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if raw.is_null() {
                return Err(io::Error::last_os_error()).context("metadata_job_failed");
            }
            let job = Self(unsafe { OwnedHandle::from_raw_handle(raw) });
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if unsafe {
                SetInformationJobObject(
                    job.0.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&limits) as u32,
                )
            } == 0
            {
                return Err(io::Error::last_os_error()).context("metadata_job_failed");
            }
            Ok(job)
        }

        pub fn attach_and_resume(&self, child: &Child) -> Result<()> {
            if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle()) }
                == 0
            {
                return Err(io::Error::last_os_error()).context("metadata_job_failed");
            }
            // std::process::Child retains the process handle but not the primary
            // thread handle. Find that still-suspended thread through ToolHelp.
            let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
            if raw == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error()).context("metadata_thread_failed");
            }
            let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
            let mut entry = THREADENTRY32 {
                dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                ..Default::default()
            };
            let mut next = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
            while next != 0 {
                if entry.th32OwnerProcessID == child.id() {
                    let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
                    if raw.is_null() {
                        return Err(io::Error::last_os_error()).context("metadata_thread_failed");
                    }
                    let thread = unsafe { OwnedHandle::from_raw_handle(raw) };
                    let count = unsafe { ResumeThread(thread.as_raw_handle()) };
                    if count == u32::MAX {
                        return Err(io::Error::last_os_error()).context("metadata_thread_failed");
                    }
                    if count > 0 {
                        ensure!(count == 1, "metadata_thread_failed");
                        return Ok(());
                    }
                }
                next = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
            }
            anyhow::bail!("metadata_thread_missing")
        }

        pub fn terminate(&self) {
            // The job handle still owns descendants after the launcher exits.
            // Dropping the handle provides an additional kill-on-close fallback.
            unsafe { TerminateJobObject(self.0.as_raw_handle(), 1) };
        }
    }
}
