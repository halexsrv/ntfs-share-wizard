use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io;
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
use crate::linux::system;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountApplyReport {
    pub success: bool,
    pub mount_command_stdout: String,
    pub mount_command_stderr: String,
    pub mountpoint_mounted: bool,
    pub read_write: bool,
    pub write_test_succeeded: bool,
    pub readonly_diagnostic: Option<String>,
    pub fast_startup_warning: Option<String>,
    pub steam_library_exists: bool,
    pub steam_library_can_create: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteamLibraryCreateReport {
    pub success: bool,
    pub summary: String,
    pub steam_library_exists: bool,
}

pub fn apply_mount_and_validate() -> MountApplyReport {
    apply_mount_and_validate_impl()
}

pub fn create_steam_library_directory() -> SteamLibraryCreateReport {
    create_steam_library_directory_impl()
}

#[cfg(target_os = "linux")]
fn apply_mount_and_validate_impl() -> MountApplyReport {
    let output = Command::new("mount").arg("-a").output();
    let (stdout, stderr) = match output {
        Ok(output) => (
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ),
        Err(error) => {
            return MountApplyReport {
                success: false,
                mount_command_stdout: String::new(),
                mount_command_stderr: error.to_string(),
                mountpoint_mounted: false,
                read_write: false,
                write_test_succeeded: false,
                readonly_diagnostic: None,
                fast_startup_warning: None,
                steam_library_exists: false,
                steam_library_can_create: false,
                summary: format!("Could not execute mount -a: {}", friendly_io_error(&error)),
            };
        }
    };

    let mount_status = inspect_mountpoint();
    let write_test = if mount_status.mounted && !mount_status.readonly {
        try_write_test()
    } else {
        WriteTestResult {
            succeeded: false,
            diagnostic: None,
        }
    };
    let steam_library_status = validate_steam_library_path();
    let readonly_diagnostic = if mount_status.mounted && mount_status.readonly {
        Some("The mountpoint is mounted but currently read-only.".to_owned())
    } else {
        write_test.diagnostic.clone()
    };
    let combined_output = format!("{stdout}\n{stderr}");
    let fast_startup_warning =
        detect_fast_startup_warning(&combined_output, readonly_diagnostic.as_deref());
    let success = mount_status.mounted && write_test.succeeded;

    MountApplyReport {
        success,
        mount_command_stdout: stdout,
        mount_command_stderr: stderr,
        mountpoint_mounted: mount_status.mounted,
        read_write: mount_status.mounted && !mount_status.readonly && write_test.succeeded,
        write_test_succeeded: write_test.succeeded,
        readonly_diagnostic,
        fast_startup_warning,
        steam_library_exists: steam_library_status.exists
            && steam_library_status.is_directory
            && !steam_library_status.is_symlink,
        steam_library_can_create: !steam_library_status.exists && steam_library_status.can_create,
        summary: if success {
            "mount -a succeeded, the mountpoint is writable, and the partition is ready for validation."
                .to_owned()
        } else if !mount_status.mounted {
            "mount -a did not leave /media/gamedisk mounted. Review the command output and fstab entry."
                .to_owned()
        } else if mount_status.readonly {
            "The partition is mounted, but only in read-only mode.".to_owned()
        } else {
            "The mountpoint was found, but the write test failed.".to_owned()
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_mount_and_validate_impl() -> MountApplyReport {
    MountApplyReport {
        success: false,
        mount_command_stdout: String::new(),
        mount_command_stderr: String::new(),
        mountpoint_mounted: false,
        read_write: false,
        write_test_succeeded: false,
        readonly_diagnostic: None,
        fast_startup_warning: None,
        steam_library_exists: false,
        steam_library_can_create: false,
        summary: "Mount validation is only available on Linux.".to_owned(),
    }
}

#[cfg(target_os = "linux")]
fn create_steam_library_directory_impl() -> SteamLibraryCreateReport {
    let validation = system::validate_mount_layout().steam_library;
    if validation.is_symlink {
        return SteamLibraryCreateReport {
            success: false,
            summary: "SteamLibrary exists as a symlink, and the wizard will not replace symlinks."
                .to_owned(),
            steam_library_exists: false,
        };
    }

    if validation.exists && !validation.is_directory {
        return SteamLibraryCreateReport {
            success: false,
            summary: "SteamLibrary exists but is not a directory.".to_owned(),
            steam_library_exists: false,
        };
    }

    match fs::create_dir_all(system::default_steam_library_path()) {
        Ok(()) => SteamLibraryCreateReport {
            success: true,
            summary: "The SteamLibrary directory is ready.".to_owned(),
            steam_library_exists: true,
        },
        Err(error) => SteamLibraryCreateReport {
            success: false,
            summary: format!(
                "Could not create the SteamLibrary directory: {}",
                friendly_io_error(&error)
            ),
            steam_library_exists: false,
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn create_steam_library_directory_impl() -> SteamLibraryCreateReport {
    SteamLibraryCreateReport {
        success: false,
        summary: "SteamLibrary creation is only available on Linux.".to_owned(),
        steam_library_exists: false,
    }
}

#[cfg(target_os = "linux")]
struct MountInspectResult {
    mounted: bool,
    readonly: bool,
}

#[cfg(target_os = "linux")]
fn inspect_mountpoint() -> MountInspectResult {
    let mountinfo = fs::read_to_string("/proc/mounts").unwrap_or_default();
    for line in mountinfo.lines() {
        let mut parts = line.split_whitespace();
        let _source = parts.next();
        let mountpoint = parts.next();
        let _fstype = parts.next();
        let options = parts.next();

        if mountpoint == Some(system::default_mountpoint()) {
            let readonly = options
                .unwrap_or_default()
                .split(',')
                .any(|option| option == "ro");
            return MountInspectResult {
                mounted: true,
                readonly,
            };
        }
    }

    MountInspectResult {
        mounted: false,
        readonly: false,
    }
}

#[cfg(target_os = "linux")]
struct WriteTestResult {
    succeeded: bool,
    diagnostic: Option<String>,
}

#[cfg(target_os = "linux")]
fn try_write_test() -> WriteTestResult {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let test_path = format!(
        "{}/.ntfs-share-wizard-write-test-{timestamp}",
        system::default_mountpoint()
    );

    match fs::write(&test_path, b"write-test") {
        Ok(()) => {
            let _ = fs::remove_file(&test_path);
            WriteTestResult {
                succeeded: true,
                diagnostic: None,
            }
        }
        Err(error) => WriteTestResult {
            succeeded: false,
            diagnostic: Some(format!("Write test failed: {}", friendly_io_error(&error))),
        },
    }
}

#[cfg(target_os = "linux")]
fn validate_steam_library_path() -> system::PathValidation {
    system::validate_mount_layout().steam_library
}

#[cfg(target_os = "linux")]
fn detect_fast_startup_warning(output: &str, diagnostic: Option<&str>) -> Option<String> {
    let combined = format!(
        "{} {}",
        output.to_ascii_lowercase(),
        diagnostic.unwrap_or_default().to_ascii_lowercase()
    );
    let suspicious = [
        "hibernat",
        "unsafe state",
        "fast startup",
        "windows is hibernated",
        "metadata kept in windows cache",
        "volume is dirty",
    ];

    suspicious.iter().any(|needle| combined.contains(needle)).then(|| {
        "The NTFS partition may be in an unsafe Windows state. Check whether Fast Startup is still enabled and perform a full Windows shutdown before retrying.".to_owned()
    })
}

#[cfg(target_os = "linux")]
fn friendly_io_error(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::PermissionDenied {
        "permission denied. Re-run the app with sufficient privileges.".to_owned()
    } else {
        error.to_string()
    }
}
