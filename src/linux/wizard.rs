use crate::app::App;
use crate::linux::system;

pub fn detected_system_details(app: &App) -> String {
    let system_info = system::inspect();
    format!(
        "Detected {} flow.\nSystem module: {}\nTarget mount point: {}",
        app.operating_system().display_name(),
        system_info.platform_label,
        system_info.fstab_mount_point
    )
}
