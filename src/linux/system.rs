use std::collections::HashMap;
use std::fs;

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{LinuxDistro, detect_distro_from_fields};

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
}
