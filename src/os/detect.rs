use crate::os::OperatingSystem;

pub fn detect_os() -> OperatingSystem {
    match std::env::consts::OS {
        "windows" => OperatingSystem::Windows,
        "linux" => OperatingSystem::Linux(crate::linux::system::detect_distro()),
        other => OperatingSystem::Unsupported(other.to_owned()),
    }
}
