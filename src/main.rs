mod app;
mod linux;
mod os;
mod tui;
mod windows;

use anyhow::Result;
use app::App;
use os::detect::detect_os;

fn main() -> Result<()> {
    let operating_system = detect_os();
    let app = App::new(operating_system);

    tui::run(app)
}
