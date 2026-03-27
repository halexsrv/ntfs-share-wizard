use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use anyhow::Context;
#[cfg(windows)]
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSystemInfo {
    pub platform_label: &'static str,
}

pub fn inspect() -> WindowsSystemInfo {
    WindowsSystemInfo {
        platform_label: "windows",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandReport {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn disable_fast_startup() -> Result<CommandReport> {
    disable_fast_startup_impl()
}

#[cfg(windows)]
fn disable_fast_startup_impl() -> Result<CommandReport> {
    let output = Command::new("powercfg")
        .args(["/h", "off"])
        .output()
        .context("failed to execute powercfg /h off")?;

    Ok(CommandReport {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

#[cfg(not(windows))]
fn disable_fast_startup_impl() -> Result<CommandReport> {
    bail!("powercfg /h off is only available on Windows")
}
