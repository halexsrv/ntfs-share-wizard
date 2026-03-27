use std::collections::HashMap;
use std::fs;
#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
use anyhow::Context;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinuxDistro {
    Ubuntu,
    SteamOS,
    Bazzite,
    Arch,
    Fedora,
    Unknown,
}

impl LinuxDistro {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Ubuntu => "Ubuntu",
            Self::SteamOS => "SteamOS",
            Self::Bazzite => "Bazzite",
            Self::Arch => "Arch Linux",
            Self::Fedora => "Fedora",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxSystemInfo {
    pub platform_label: &'static str,
    pub fstab_mount_point: &'static str,
    pub distro: LinuxDistro,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NtfsPartition {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub label: Option<String>,
    pub uuid: String,
    pub mountpoint: Option<String>,
}

pub fn inspect(distro: LinuxDistro) -> LinuxSystemInfo {
    LinuxSystemInfo {
        platform_label: "linux",
        fstab_mount_point: "/media/gamedisk",
        distro,
    }
}

pub fn detect_distro() -> LinuxDistro {
    let Ok(os_release) = fs::read_to_string("/etc/os-release") else {
        return LinuxDistro::Unknown;
    };

    let fields = parse_os_release(&os_release);
    detect_distro_from_fields(&fields)
}

pub fn detect_ntfs_partitions() -> Result<Vec<NtfsPartition>> {
    detect_ntfs_partitions_impl()
}

pub fn human_readable_size(size_bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = size_bytes as f64;
    let mut unit_index = 0usize;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{size_bytes} {}", UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

pub fn friendly_partition_title(partition: &NtfsPartition) -> String {
    let label = partition
        .label
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Sem label");

    format!(
        "{} ({}, {}, UUID {})",
        label,
        partition.path,
        human_readable_size(partition.size_bytes),
        partition.uuid
    )
}

#[cfg(target_os = "linux")]
fn detect_ntfs_partitions_impl() -> Result<Vec<NtfsPartition>> {
    let output = Command::new("lsblk")
        .args([
            "-b",
            "-J",
            "-o",
            "NAME,PATH,SIZE,FSTYPE,LABEL,UUID,MOUNTPOINT,TYPE",
        ])
        .output()
        .context("failed to execute lsblk")?;

    if !output.status.success() {
        bail!(
            "lsblk failed with status {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let response: LsblkResponse =
        serde_json::from_slice(&output.stdout).context("failed to parse lsblk JSON output")?;

    Ok(ntfs_partitions_from_response(response))
}

#[cfg(not(target_os = "linux"))]
fn detect_ntfs_partitions_impl() -> Result<Vec<NtfsPartition>> {
    bail!("lsblk partition detection is only available on Linux")
}

fn ntfs_partitions_from_response(response: LsblkResponse) -> Vec<NtfsPartition> {
    let mut flattened = Vec::new();
    for device in response.blockdevices {
        flatten_device_tree(device, &mut flattened);
    }

    let mut partitions: Vec<_> = flattened
        .into_iter()
        .filter_map(|device| {
            let kind = device.device_type?;
            let fstype = device.fstype?;
            let uuid = non_empty(device.uuid)?;
            let path = non_empty(device.path)?;

            if kind != "part" || !fstype.eq_ignore_ascii_case("ntfs") {
                return None;
            }

            Some(NtfsPartition {
                name: device.name.unwrap_or_else(|| path.clone()),
                path,
                size_bytes: device.size.unwrap_or(0),
                label: non_empty(device.label),
                uuid,
                mountpoint: non_empty(device.mountpoint),
            })
        })
        .collect();

    partitions.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));
    partitions
}

fn flatten_device_tree(device: LsblkDevice, output: &mut Vec<LsblkDevice>) {
    let children = device.children.clone().unwrap_or_default();
    output.push(LsblkDevice {
        children: None,
        ..device
    });

    for child in children {
        flatten_device_tree(child, output);
    }
}

fn detect_distro_from_fields(fields: &HashMap<String, String>) -> LinuxDistro {
    let id = field_value(fields, "ID");
    let id_like = field_value(fields, "ID_LIKE");
    let name = field_value(fields, "NAME");
    let pretty_name = field_value(fields, "PRETTY_NAME");
    let combined = [
        id.as_str(),
        id_like.as_str(),
        name.as_str(),
        pretty_name.as_str(),
    ]
    .join(" ");
    let normalized = combined.to_ascii_lowercase();

    if contains_any(&normalized, &["steamos"]) {
        LinuxDistro::SteamOS
    } else if contains_any(&normalized, &["bazzite"]) {
        LinuxDistro::Bazzite
    } else if contains_any(&normalized, &["ubuntu"]) {
        LinuxDistro::Ubuntu
    } else if contains_any(&normalized, &["arch"]) {
        LinuxDistro::Arch
    } else if contains_any(&normalized, &["fedora"]) {
        LinuxDistro::Fedora
    } else {
        LinuxDistro::Unknown
    }
}

fn parse_os_release(contents: &str) -> HashMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let (key, value) = line.split_once('=')?;
            Some((key.to_owned(), unquote(value.trim())))
        })
        .collect()
}

fn field_value(fields: &HashMap<String, String>, key: &str) -> String {
    fields.get(key).cloned().unwrap_or_default()
}

fn unquote(value: &str) -> String {
    value.trim_matches('"').trim_matches('\'').to_owned()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

#[derive(Debug, Clone, Deserialize)]
struct LsblkResponse {
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Debug, Clone, Deserialize)]
struct LsblkDevice {
    name: Option<String>,
    path: Option<String>,
    size: Option<u64>,
    fstype: Option<String>,
    label: Option<String>,
    uuid: Option<String>,
    mountpoint: Option<String>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    children: Option<Vec<LsblkDevice>>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        LinuxDistro, LsblkDevice, LsblkResponse, NtfsPartition, detect_distro_from_fields,
        friendly_partition_title, human_readable_size, ntfs_partitions_from_response,
    };

    #[test]
    fn prioritizes_steamos_over_arch_like() {
        let fields = HashMap::from([
            ("ID".to_owned(), "steamos".to_owned()),
            ("ID_LIKE".to_owned(), "arch".to_owned()),
        ]);

        assert_eq!(detect_distro_from_fields(&fields), LinuxDistro::SteamOS);
    }

    #[test]
    fn prioritizes_bazzite_over_fedora_like() {
        let fields = HashMap::from([
            ("ID".to_owned(), "bazzite".to_owned()),
            ("ID_LIKE".to_owned(), "fedora".to_owned()),
        ]);

        assert_eq!(detect_distro_from_fields(&fields), LinuxDistro::Bazzite);
    }

    #[test]
    fn filters_and_sorts_ntfs_partitions_by_size_descending() {
        let response = LsblkResponse {
            blockdevices: vec![LsblkDevice {
                name: Some("nvme0n1".to_owned()),
                path: Some("/dev/nvme0n1".to_owned()),
                size: Some(2_000_000_000_000),
                fstype: None,
                label: None,
                uuid: None,
                mountpoint: None,
                device_type: Some("disk".to_owned()),
                children: Some(vec![
                    LsblkDevice {
                        name: Some("nvme0n1p1".to_owned()),
                        path: Some("/dev/nvme0n1p1".to_owned()),
                        size: Some(1_000_000_000_000),
                        fstype: Some("ntfs".to_owned()),
                        label: Some("Games".to_owned()),
                        uuid: Some("UUID-GAMES".to_owned()),
                        mountpoint: None,
                        device_type: Some("part".to_owned()),
                        children: None,
                    },
                    LsblkDevice {
                        name: Some("nvme0n1p2".to_owned()),
                        path: Some("/dev/nvme0n1p2".to_owned()),
                        size: Some(500_000_000_000),
                        fstype: Some("ext4".to_owned()),
                        label: Some("Linux".to_owned()),
                        uuid: Some("UUID-LINUX".to_owned()),
                        mountpoint: Some("/".to_owned()),
                        device_type: Some("part".to_owned()),
                        children: None,
                    },
                    LsblkDevice {
                        name: Some("nvme0n1p3".to_owned()),
                        path: Some("/dev/nvme0n1p3".to_owned()),
                        size: Some(750_000_000_000),
                        fstype: Some("NTFS".to_owned()),
                        label: None,
                        uuid: Some("UUID-BACKUP".to_owned()),
                        mountpoint: Some("/mnt/backup".to_owned()),
                        device_type: Some("part".to_owned()),
                        children: None,
                    },
                    LsblkDevice {
                        name: Some("nvme0n1p4".to_owned()),
                        path: Some("/dev/nvme0n1p4".to_owned()),
                        size: Some(250_000_000_000),
                        fstype: Some("ntfs".to_owned()),
                        label: Some("MissingUuid".to_owned()),
                        uuid: None,
                        mountpoint: None,
                        device_type: Some("part".to_owned()),
                        children: None,
                    },
                ]),
            }],
        };

        let partitions = ntfs_partitions_from_response(response);

        assert_eq!(
            partitions,
            vec![
                NtfsPartition {
                    name: "nvme0n1p1".to_owned(),
                    path: "/dev/nvme0n1p1".to_owned(),
                    size_bytes: 1_000_000_000_000,
                    label: Some("Games".to_owned()),
                    uuid: "UUID-GAMES".to_owned(),
                    mountpoint: None,
                },
                NtfsPartition {
                    name: "nvme0n1p3".to_owned(),
                    path: "/dev/nvme0n1p3".to_owned(),
                    size_bytes: 750_000_000_000,
                    label: None,
                    uuid: "UUID-BACKUP".to_owned(),
                    mountpoint: Some("/mnt/backup".to_owned()),
                },
            ]
        );
    }

    #[test]
    fn formats_human_readable_sizes() {
        assert_eq!(human_readable_size(999), "999 B");
        assert_eq!(human_readable_size(1024), "1.0 KiB");
        assert_eq!(human_readable_size(1024 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn builds_friendly_partition_titles() {
        let partition = NtfsPartition {
            name: "sda1".to_owned(),
            path: "/dev/sda1".to_owned(),
            size_bytes: 1_099_511_627_776,
            label: Some("Games".to_owned()),
            uuid: "ABCD-1234".to_owned(),
            mountpoint: None,
        };

        assert_eq!(
            friendly_partition_title(&partition),
            "Games (/dev/sda1, 1.0 TiB, UUID ABCD-1234)"
        );
    }
}
