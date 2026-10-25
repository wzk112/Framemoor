use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{fs::File, io, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootIdentity {
    pub volume: String,
    pub relative_directory: String,
    pub filesystem: String,
}

pub fn root_identity(path: &Path) -> Result<RootIdentity> {
    native::root_identity(path)
}

pub fn available_space(path: &Path) -> Result<u64> {
    native::available_space(path)
}

pub fn physical_memory() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        let mut bytes = 0u64;
        let mut length = std::mem::size_of::<u64>();
        let result = unsafe {
            libc::sysctlbyname(
                c"hw.memsize".as_ptr(),
                (&mut bytes as *mut u64).cast(),
                &mut length,
                std::ptr::null_mut(),
                0,
            )
        };
        (result == 0 && length == std::mem::size_of::<u64>()).then_some(bytes)
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (unsafe { GlobalMemoryStatusEx(&mut status) } != 0).then_some(status.ullTotalPhys)
    }
}

pub fn publish(source: &Path, target: &Path) -> Result<()> {
    native::rename(source, target, false)
        .context("Could not publish without replacing an existing file")
}

pub fn unsupported(error: &anyhow::Error) -> bool {
    error.downcast_ref::<io::Error>().is_some_and(|e| {
        if e.kind() == io::ErrorKind::Unsupported {
            return true;
        }
        #[cfg(target_os = "macos")]
        {
            matches!(
                e.raw_os_error(),
                Some(libc::ENOTSUP) | Some(libc::EOPNOTSUPP)
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    })
}

pub fn file_identity(path: &Path) -> Result<String> {
    handle_identity(&File::open(path)?)
}

pub fn handle_identity(file: &File) -> Result<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata()?;
        let birth = meta
            .created()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        // macOS ExFAT gives empty files transient vnode IDs; no content can be
        // resumed for an empty file. Creation time is stable across reopening.
        if meta.len() == 0 {
            return Ok(format!("empty:{birth}"));
        }
        Ok(format!("{}:{birth}", meta.ino()))
    }
    #[cfg(windows)]
    {
        native::handle_identity(file)
    }
}

pub fn replace_journal(source: &Path, target: &Path) -> Result<()> {
    native::rename(source, target, true).context("Could not replace task journal")
}

pub fn sync_directory(path: &Path) -> Result<()> {
    native::sync_directory(path)
}

pub fn sync_file(file: &File) -> Result<()> {
    file.sync_all()?;
    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        // Request a device-cache flush as well as the ordinary fsync.
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_FULLFSYNC) } == -1 {
            return Err(io::Error::last_os_error()).context("Full file flush failed");
        }
    }
    Ok(())
}

pub fn mount_token(path: &Path) -> Result<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(path.metadata()?.dev())
    }
    #[cfg(windows)]
    {
        native::serial(path).map(u64::from)
    }
}

pub fn reject_link(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                bail!("Symbolic links are not supported: {}", path.display());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    bail!(
                        "Windows reparse points are not supported: {}",
                        path.display()
                    );
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::{
        ffi::{CStr, CString},
        os::unix::ffi::OsStrExt,
        process::Command,
    };

    fn cpath(path: &Path) -> Result<CString> {
        Ok(CString::new(path.as_os_str().as_bytes())?)
    }

    pub fn root_identity(path: &Path) -> Result<RootIdentity> {
        let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
        if unsafe { libc::statfs(cpath(path)?.as_ptr(), stat.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        let stat = unsafe { stat.assume_init() };
        let mount = unsafe { CStr::from_ptr(stat.f_mntonname.as_ptr()) }.to_str()?;
        let output = Command::new("/usr/sbin/diskutil")
            .args(["info", "-plist", mount])
            .output()?;
        if !output.status.success() {
            bail!("Unable to identify mounted volume");
        }
        let plist = plist::Value::from_reader(std::io::Cursor::new(output.stdout))?;
        let dict = plist.as_dictionary().context("Invalid diskutil result")?;
        let volume = dict
            .get("VolumeUUID")
            .and_then(plist::Value::as_string)
            .context("Volume UUID is unavailable; refusing unsafe resume")?;
        let filesystem = dict
            .get("FilesystemType")
            .and_then(plist::Value::as_string)
            .unwrap_or("unknown");
        // /Users is a firmlink into the APFS Data volume; normalize both spellings.
        let relative = path
            .strip_prefix(mount)
            .unwrap_or(path)
            .to_string_lossy()
            .trim_start_matches('/')
            .to_owned();
        Ok(RootIdentity {
            volume: volume.to_owned(),
            relative_directory: relative,
            filesystem: filesystem.to_owned(),
        })
    }

    pub fn rename(source: &Path, target: &Path, replace: bool) -> Result<()> {
        let flags = if replace { 0 } else { libc::RENAME_EXCL };
        let result =
            unsafe { libc::renamex_np(cpath(source)?.as_ptr(), cpath(target)?.as_ptr(), flags) };
        if result == -1 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub fn sync_directory(path: &Path) -> Result<()> {
        File::open(path)?.sync_all()?;
        Ok(())
    }

    pub fn available_space(path: &Path) -> Result<u64> {
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(cpath(path)?.as_ptr(), stat.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        let stat = unsafe { stat.assume_init() };
        Ok(u64::from(stat.f_bavail).saturating_mul(stat.f_frsize))
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::{
        fs::OpenOptions,
        os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    };
    use windows_sys::Win32::Storage::FileSystem::*;

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    pub fn serial(path: &Path) -> Result<u32> {
        let file = OpenOptions::new()
            .access_mode(0)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(unsafe { info.assume_init() }.dwVolumeSerialNumber)
    }

    pub fn handle_identity(file: &File) -> Result<String> {
        let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
            return Err(io::Error::last_os_error().into());
        }
        let info = unsafe { info.assume_init() };
        Ok(format!(
            "{}:{}:{}:{}:{}",
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
            info.ftCreationTime.dwHighDateTime,
            info.ftCreationTime.dwLowDateTime
        ))
    }

    pub fn root_identity(path: &Path) -> Result<RootIdentity> {
        let mut mount = vec![0u16; 32768];
        if unsafe {
            GetVolumePathNameW(wide(path).as_ptr(), mount.as_mut_ptr(), mount.len() as u32)
        } == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let mut fs = [0u16; 128];
        if unsafe {
            GetVolumeInformationW(
                mount.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                fs.as_mut_ptr(),
                fs.len() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let mount_str = String::from_utf16(&mount[..mount.iter().position(|x| *x == 0).unwrap()])?;
        let filesystem = String::from_utf16(&fs[..fs.iter().position(|x| *x == 0).unwrap()])?;
        let relative = path
            .strip_prefix(Path::new(&mount_str))
            .context("Cannot resolve volume-relative path")?
            .to_string_lossy()
            .replace('\\', "/");
        Ok(RootIdentity {
            volume: format!("{:08x}", serial(path)?),
            relative_directory: relative,
            filesystem,
        })
    }

    pub fn rename(source: &Path, target: &Path, replace: bool) -> Result<()> {
        let flags = MOVEFILE_WRITE_THROUGH
            | if replace {
                MOVEFILE_REPLACE_EXISTING
            } else {
                0
            };
        if unsafe { MoveFileExW(wide(source).as_ptr(), wide(target).as_ptr(), flags) } == 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub fn sync_directory(_path: &Path) -> Result<()> {
        // Windows has no equivalent general directory fsync here. Files are flushed;
        // metadata updates use MoveFileExW. Power-loss durability is not certified.
        Ok(())
    }

    pub fn available_space(path: &Path) -> Result<u64> {
        let mut free = 0;
        if unsafe {
            GetDiskFreeSpaceExW(
                wide(path).as_ptr(),
                &mut free,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        Ok(free)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
compile_error!("This milestone supports macOS and Windows only.");
