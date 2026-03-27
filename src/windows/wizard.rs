use anyhow::Result;

use crate::app::App;
use crate::tui;
use crate::windows::system;

pub fn run(app: &App) -> Result<()> {
    let system_info = system::inspect();
    let details = format!(
        "Detected {} flow.\nSystem module: {}",
        app.operating_system().display_name(),
        system_info.platform_label
    );

    tui::render_once(app, "Windows Wizard", &details)
}
