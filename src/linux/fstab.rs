use serde::{Deserialize, Serialize};

use crate::linux::system::{self, NtfsPartition};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io;
#[cfg(target_os = "linux")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
const FSTAB_PATH: &str = "/etc/fstab";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FstabWriteReport {
    pub success: bool,
    pub backup_path: Option<String>,
    pub written_line: String,
    pub entry_already_exists: bool,
    pub summary: String,
}

pub fn write_entry(partition: &NtfsPartition) -> FstabWriteReport {
    write_entry_impl(partition)
}

#[cfg(target_os = "linux")]
fn write_entry_impl(partition: &NtfsPartition) -> FstabWriteReport {
    let written_line = system::generate_fstab_entry(partition);
    let fstab_contents = match fs::read_to_string(FSTAB_PATH) {
        Ok(contents) => contents,
        Err(error) => {
            return failure_report(
                written_line,
                None,
                false,
                format!("Could not read /etc/fstab: {}", friendly_io_error(&error)),
            );
        }
    };

    if contains_uuid_entry(&fstab_contents, &partition.uuid) {
        return FstabWriteReport {
            success: true,
            backup_path: None,
            written_line,
            entry_already_exists: true,
            summary:
                "An entry for this UUID already exists in /etc/fstab. No duplicate was written."
                    .to_owned(),
        };
    }

    if let Err(error) = fs::create_dir_all(system::default_mountpoint()) {
        return failure_report(
            written_line,
            None,
            false,
            format!(
                "Could not ensure the mountpoint exists: {}",
                friendly_io_error(&error)
            ),
        );
    }

    let backup_path = backup_path();
    if let Err(error) = fs::copy(FSTAB_PATH, &backup_path) {
        return failure_report(
            written_line,
            None,
            false,
            format!(
                "Could not create the /etc/fstab backup: {}",
                friendly_io_error(&error)
            ),
        );
    }

    let mut updated = fstab_contents;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&written_line);
    updated.push('\n');

    match fs::write(FSTAB_PATH, updated) {
        Ok(()) => FstabWriteReport {
            success: true,
            backup_path: Some(backup_path),
            written_line,
            entry_already_exists: false,
            summary: "The new NTFS entry was appended to /etc/fstab after creating a backup."
                .to_owned(),
        },
        Err(error) => failure_report(
            written_line,
            Some(backup_path),
            false,
            format!("Could not write /etc/fstab: {}", friendly_io_error(&error)),
        ),
    }
}

#[cfg(not(target_os = "linux"))]
fn write_entry_impl(partition: &NtfsPartition) -> FstabWriteReport {
    FstabWriteReport {
        success: false,
        backup_path: None,
        written_line: system::generate_fstab_entry(partition),
        entry_already_exists: false,
        summary: "Safe fstab writing is only available on Linux.".to_owned(),
    }
}

#[cfg(any(test, target_os = "linux"))]
fn contains_uuid_entry(contents: &str, uuid: &str) -> bool {
    let needle = format!("UUID={uuid}");
    contents.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.contains(&needle)
    })
}

#[cfg(target_os = "linux")]
fn backup_path() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("/etc/fstab.backup.{timestamp}")
}

#[cfg(target_os = "linux")]
fn failure_report(
    written_line: String,
    backup_path: Option<String>,
    entry_already_exists: bool,
    summary: String,
) -> FstabWriteReport {
    FstabWriteReport {
        success: false,
        backup_path,
        written_line,
        entry_already_exists,
        summary,
    }
}

#[cfg(target_os = "linux")]
fn friendly_io_error(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::PermissionDenied {
        "permission denied. Re-run the app with sufficient privileges.".to_owned()
    } else {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::contains_uuid_entry;

    #[test]
    fn detects_existing_uuid_without_matching_comments() {
        let contents = "\
# UUID=OLD-COMMENT /tmp ntfs-3g defaults 0 0
UUID=REAL-UUID /media/gamedisk ntfs-3g uid=1000,gid=1000,rw,noatime,user,exec,umask=022,nofail 0 0
";

        assert!(contains_uuid_entry(contents, "REAL-UUID"));
        assert!(!contains_uuid_entry(contents, "OLD-COMMENT"));
    }
}
