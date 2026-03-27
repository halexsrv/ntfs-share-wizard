use crate::app::App;
use crate::windows::system;

pub fn detected_system_details(app: &App) -> String {
    let system_info = system::inspect();
    format!(
        "Detected {} flow.\nSystem module: {}",
        app.operating_system().display_name(),
        system_info.platform_label
    )
}
