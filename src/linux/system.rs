use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxSystemInfo {
    pub platform_label: &'static str,
    pub fstab_mount_point: &'static str,
}

pub fn inspect() -> LinuxSystemInfo {
    LinuxSystemInfo {
        platform_label: "linux",
        fstab_mount_point: "/media/gamedisk",
    }
}
