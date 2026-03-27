use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallPlan {
    pub title: String,
    pub steps: Vec<InstallStep>,
    pub caution: Option<String>,
    pub execution_mode: InstallExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallStep {
    pub label: String,
    pub command_preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallExecutionMode {
    Assisted,
    GuidedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallCommandResult {
    pub label: String,
    pub command: Option<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub skipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallExecutionReport {
    pub success: bool,
    pub final_ntfs_3g_installed: bool,
    pub summary: String,
    pub command_results: Vec<InstallCommandResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathValidation {
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub can_create: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountLayoutStatus {
    pub mountpoint: PathValidation,
    pub steam_library: PathValidation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathCreationReport {
    pub success: bool,
    pub created_anything: bool,
    pub summary: String,
    pub mount_layout: MountLayoutStatus,
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

pub fn is_ntfs_3g_installed() -> bool {
    executable_in_path("ntfs-3g")
}

pub fn install_plan_for_distro(distro: &LinuxDistro) -> InstallPlan {
    match distro {
        LinuxDistro::Ubuntu => InstallPlan {
            title: "Ubuntu install plan".to_owned(),
            steps: vec![
                InstallStep {
                    label: "Refresh apt metadata".to_owned(),
                    command_preview: Some("sudo apt update".to_owned()),
                },
                InstallStep {
                    label: "Install ntfs-3g".to_owned(),
                    command_preview: Some("sudo apt install -y ntfs-3g".to_owned()),
                },
            ],
            caution: None,
            execution_mode: InstallExecutionMode::Assisted,
        },
        LinuxDistro::SteamOS => InstallPlan {
            title: "SteamOS install plan".to_owned(),
            steps: vec![
                InstallStep {
                    label: "Disable readonly mode".to_owned(),
                    command_preview: Some("sudo steamos-readonly disable".to_owned()),
                },
                InstallStep {
                    label: "Install ntfs-3g".to_owned(),
                    command_preview: Some("sudo pacman -Sy --noconfirm ntfs-3g".to_owned()),
                },
                InstallStep {
                    label: "Re-enable readonly mode".to_owned(),
                    command_preview: Some("sudo steamos-readonly enable".to_owned()),
                },
            ],
            caution: Some(
                "SteamOS uses a readonly base image. Re-enable readonly mode after installing packages."
                    .to_owned(),
            ),
            execution_mode: InstallExecutionMode::Assisted,
        },
        LinuxDistro::Bazzite => InstallPlan {
            title: "Bazzite install plan".to_owned(),
            steps: vec![
                InstallStep {
                    label: "Review the supported Bazzite workflow".to_owned(),
                    command_preview: None,
                },
                InstallStep {
                    label: "Confirm whether ntfs-3g should come from the image, toolbox, or rpm-ostree layering".to_owned(),
                    command_preview: None,
                },
                InstallStep {
                    label: "Avoid changing the immutable base until the exact supported path is confirmed".to_owned(),
                    command_preview: None,
                },
            ],
            caution: Some(
                "Bazzite is Fedora Atomic-based, so package installation needs extra care before changing the host."
                    .to_owned(),
            ),
            execution_mode: InstallExecutionMode::GuidedOnly,
        },
        LinuxDistro::Arch => InstallPlan {
            title: "Arch Linux install plan".to_owned(),
            steps: vec![
                InstallStep {
                    label: "Refresh pacman metadata".to_owned(),
                    command_preview: Some("sudo pacman -Sy".to_owned()),
                },
                InstallStep {
                    label: "Install ntfs-3g".to_owned(),
                    command_preview: Some("sudo pacman -S --noconfirm ntfs-3g".to_owned()),
                },
            ],
            caution: None,
            execution_mode: InstallExecutionMode::Assisted,
        },
        LinuxDistro::Fedora => InstallPlan {
            title: "Fedora install plan".to_owned(),
            steps: vec![InstallStep {
                label: "Install ntfs-3g".to_owned(),
                command_preview: Some("sudo dnf install -y ntfs-3g".to_owned()),
            }],
            caution: None,
            execution_mode: InstallExecutionMode::Assisted,
        },
        LinuxDistro::Unknown => InstallPlan {
            title: "Unknown distro install plan".to_owned(),
            steps: vec![
                InstallStep {
                    label: "Identify your distro package manager".to_owned(),
                    command_preview: None,
                },
                InstallStep {
                    label: "Install the ntfs-3g package using the distro-supported workflow".to_owned(),
                    command_preview: None,
                },
            ],
            caution: Some(
                "The distro could not be identified automatically, so confirm the correct package source before installing."
                    .to_owned(),
            ),
            execution_mode: InstallExecutionMode::GuidedOnly,
        },
    }
}

pub fn execute_install_plan(distro: &LinuxDistro) -> InstallExecutionReport {
    execute_install_plan_impl(distro)
}

pub fn default_mountpoint() -> &'static str {
    "/media/gamedisk"
}

pub fn default_steam_library_path() -> &'static str {
    "/media/gamedisk/SteamLibrary"
}

pub fn validate_mount_layout() -> MountLayoutStatus {
    MountLayoutStatus {
        mountpoint: validate_path(default_mountpoint()),
        steam_library: validate_path(default_steam_library_path()),
    }
}

pub fn create_missing_mount_layout() -> PathCreationReport {
    create_missing_mount_layout_impl()
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

#[cfg(any(test, target_os = "linux"))]
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

#[cfg(any(test, target_os = "linux"))]
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

fn executable_in_path(binary_name: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths)
        .map(|path| path.join(binary_name))
        .any(|candidate| is_executable_file(&candidate))
}

fn is_executable_file(path: &PathBuf) -> bool {
    path.is_file()
}

fn validate_path(path: &str) -> PathValidation {
    let path_ref = Path::new(path);
    let symlink_metadata = fs::symlink_metadata(path_ref).ok();
    let exists = symlink_metadata.is_some();
    let is_symlink = symlink_metadata
        .as_ref()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    let is_directory = symlink_metadata
        .as_ref()
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false);
    let can_create = !exists && path_ref.parent().map(Path::exists).unwrap_or(false);

    PathValidation {
        path: path.to_owned(),
        exists,
        is_directory,
        is_symlink,
        can_create,
    }
}

#[cfg(target_os = "linux")]
fn create_missing_mount_layout_impl() -> PathCreationReport {
    let initial = validate_mount_layout();

    if initial.mountpoint.is_symlink || initial.steam_library.is_symlink {
        return PathCreationReport {
            success: false,
            created_anything: false,
            summary:
                "A symlink was found in the target path. The wizard will not replace symlinks."
                    .to_owned(),
            mount_layout: initial,
        };
    }

    if initial.mountpoint.exists && !initial.mountpoint.is_directory {
        return PathCreationReport {
            success: false,
            created_anything: false,
            summary: "The mountpoint path exists but is not a directory.".to_owned(),
            mount_layout: initial,
        };
    }

    if initial.steam_library.exists && !initial.steam_library.is_directory {
        return PathCreationReport {
            success: false,
            created_anything: false,
            summary: "The SteamLibrary path exists but is not a directory.".to_owned(),
            mount_layout: initial,
        };
    }

    let created_anything = !initial.mountpoint.exists || !initial.steam_library.exists;
    let create_result = fs::create_dir_all(default_steam_library_path());
    let final_layout = validate_mount_layout();

    match create_result {
        Ok(()) => PathCreationReport {
            success: final_layout.mountpoint.exists
                && final_layout.mountpoint.is_directory
                && final_layout.steam_library.exists
                && final_layout.steam_library.is_directory
                && !final_layout.mountpoint.is_symlink
                && !final_layout.steam_library.is_symlink,
            created_anything,
            summary: "The default mountpoint layout is ready.".to_owned(),
            mount_layout: final_layout,
        },
        Err(error) => PathCreationReport {
            success: false,
            created_anything: false,
            summary: format!("Could not create the default mount layout: {error}"),
            mount_layout: final_layout,
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn create_missing_mount_layout_impl() -> PathCreationReport {
    PathCreationReport {
        success: false,
        created_anything: false,
        summary: "Mount layout creation is only available on Linux.".to_owned(),
        mount_layout: validate_mount_layout(),
    }
}

#[cfg(target_os = "linux")]
fn execute_install_plan_impl(distro: &LinuxDistro) -> InstallExecutionReport {
    match distro {
        LinuxDistro::Ubuntu => execute_command_sequence(vec![
            ("Refresh apt metadata", vec!["sudo", "apt", "update"]),
            (
                "Install ntfs-3g",
                vec!["sudo", "apt", "install", "-y", "ntfs-3g"],
            ),
        ]),
        LinuxDistro::Arch => execute_command_sequence(vec![
            ("Refresh pacman metadata", vec!["sudo", "pacman", "-Sy"]),
            (
                "Install ntfs-3g",
                vec!["sudo", "pacman", "-S", "--noconfirm", "ntfs-3g"],
            ),
        ]),
        LinuxDistro::Fedora => execute_command_sequence(vec![(
            "Install ntfs-3g",
            vec!["sudo", "dnf", "install", "-y", "ntfs-3g"],
        )]),
        LinuxDistro::SteamOS => execute_steamos_install_sequence(),
        LinuxDistro::Bazzite => InstallExecutionReport {
            success: false,
            final_ntfs_3g_installed: is_ntfs_3g_installed(),
            summary: "Bazzite requires a conservative guided flow. No automatic commands were executed."
                .to_owned(),
            command_results: Vec::new(),
        },
        LinuxDistro::Unknown => InstallExecutionReport {
            success: false,
            final_ntfs_3g_installed: is_ntfs_3g_installed(),
            summary:
                "The distro could not be identified safely, so no installation commands were executed."
                    .to_owned(),
            command_results: Vec::new(),
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn execute_install_plan_impl(_distro: &LinuxDistro) -> InstallExecutionReport {
    InstallExecutionReport {
        success: false,
        final_ntfs_3g_installed: false,
        summary: "Assisted ntfs-3g installation is only available on Linux.".to_owned(),
        command_results: Vec::new(),
    }
}

#[cfg(target_os = "linux")]
fn execute_command_sequence(steps: Vec<(&str, Vec<&str>)>) -> InstallExecutionReport {
    let mut command_results = Vec::new();

    for (label, command) in steps {
        let result = run_command(label, &command);
        let should_continue = result.success;
        command_results.push(result);

        if !should_continue {
            let final_ntfs_3g_installed = is_ntfs_3g_installed();
            return InstallExecutionReport {
                success: false,
                final_ntfs_3g_installed,
                summary: "The installation stopped because one command failed.".to_owned(),
                command_results,
            };
        }
    }

    let final_ntfs_3g_installed = is_ntfs_3g_installed();
    InstallExecutionReport {
        success: final_ntfs_3g_installed,
        final_ntfs_3g_installed,
        summary: if final_ntfs_3g_installed {
            "The assisted ntfs-3g installation completed successfully.".to_owned()
        } else {
            "The commands completed, but ntfs-3g is still missing from PATH.".to_owned()
        },
        command_results,
    }
}

#[cfg(target_os = "linux")]
fn execute_steamos_install_sequence() -> InstallExecutionReport {
    let disable = run_command(
        "Disable readonly mode",
        &["sudo", "steamos-readonly", "disable"],
    );
    let disable_succeeded = disable.success;
    let mut command_results = vec![disable];

    if disable_succeeded {
        let install = run_command(
            "Install ntfs-3g",
            &["sudo", "pacman", "-Sy", "--noconfirm", "ntfs-3g"],
        );
        let install_succeeded = install.success;
        command_results.push(install);

        let enable = run_command(
            "Re-enable readonly mode",
            &["sudo", "steamos-readonly", "enable"],
        );
        let enable_succeeded = enable.success;
        command_results.push(enable);

        let final_ntfs_3g_installed = is_ntfs_3g_installed();
        let success = install_succeeded && enable_succeeded && final_ntfs_3g_installed;
        return InstallExecutionReport {
            success,
            final_ntfs_3g_installed,
            summary: if success {
                "SteamOS readonly mode was restored and ntfs-3g is now available.".to_owned()
            } else {
                "SteamOS installation finished with errors. Review the command results before continuing."
                    .to_owned()
            },
            command_results,
        };
    }

    let final_ntfs_3g_installed = is_ntfs_3g_installed();
    InstallExecutionReport {
        success: false,
        final_ntfs_3g_installed,
        summary: "SteamOS readonly mode could not be disabled, so installation was not attempted."
            .to_owned(),
        command_results,
    }
}

#[cfg(target_os = "linux")]
fn run_command(label: &str, command: &[&str]) -> InstallCommandResult {
    let output = match Command::new(command[0]).args(&command[1..]).output() {
        Ok(output) => output,
        Err(error) => {
            return InstallCommandResult {
                label: label.to_owned(),
                command: Some(command.join(" ")),
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
                skipped: false,
            };
        }
    };

    InstallCommandResult {
        label: label.to_owned(),
        command: Some(command.join(" ")),
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        skipped: false,
    }
}

#[cfg(any(test, target_os = "linux"))]
fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

#[cfg(any(test, target_os = "linux"))]
#[derive(Debug, Clone, Deserialize)]
struct LsblkResponse {
    blockdevices: Vec<LsblkDevice>,
}

#[cfg(any(test, target_os = "linux"))]
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
        InstallPlan, LinuxDistro, LsblkDevice, LsblkResponse, NtfsPartition,
        detect_distro_from_fields, friendly_partition_title, human_readable_size,
        install_plan_for_distro, ntfs_partitions_from_response,
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

    #[test]
    fn creates_steamos_install_plan_with_readonly_steps() {
        let plan = install_plan_for_distro(&LinuxDistro::SteamOS);

        assert_eq!(plan.title, "SteamOS install plan");
        assert_eq!(
            plan.steps,
            vec![
                "sudo steamos-readonly disable",
                "sudo pacman -Sy ntfs-3g",
                "sudo steamos-readonly enable",
            ]
        );
        assert!(plan.caution.is_some());
    }

    #[test]
    fn creates_bazzite_install_plan_with_atomic_caution() {
        let plan = install_plan_for_distro(&LinuxDistro::Bazzite);

        assert_eq!(plan.title, "Bazzite install plan");
        assert!(plan.steps.len() >= 2);
        assert!(
            plan.caution
                .as_deref()
                .unwrap_or_default()
                .contains("Fedora Atomic")
        );
    }
}
