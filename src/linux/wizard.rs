use crate::app::App;
use crate::linux::system;
use crate::os::OperatingSystem;

pub fn detected_system_details(app: &App) -> String {
    let distro = match app.operating_system() {
        OperatingSystem::Linux(distro) => distro.clone(),
        _ => system::LinuxDistro::Unknown,
    };
    let system_info = system::inspect(distro);
    format!(
        "Detected {} flow.\nLinux distro: {}\nSystem module: {}\nTarget mount point: {}",
        app.operating_system().display_name(),
        system_info.distro.display_name(),
        system_info.platform_label,
        system_info.fstab_mount_point
    )
}
