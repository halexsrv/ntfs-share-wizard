mod app;
mod linux;
mod os;
mod tui;
mod windows;

use anyhow::Result;
use app::App;
use os::OperatingSystem;
use os::detect::detect_os;

fn main() -> Result<()> {
    let operating_system = detect_os();
    let app = App::new(operating_system.clone());

    match operating_system {
        OperatingSystem::Windows => windows::wizard::run(&app),
        OperatingSystem::Linux => linux::wizard::run(&app),
        OperatingSystem::Unsupported(name) => {
            anyhow::bail!("unsupported operating system: {name}")
        }
    }
}
