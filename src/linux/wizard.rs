use anyhow::Result;

use crate::app::App;
use crate::linux::system;
use crate::tui;

pub fn run(app: &App) -> Result<()> {
    let system_info = system::inspect();
    let details = format!(
        "Detected {} flow.\nSystem module: {}\nTarget mount point: {}",
        app.operating_system().display_name(),
        system_info.platform_label,
        system_info.fstab_mount_point
    );

    tui::render_once(app, "Linux Wizard", &details)
}
