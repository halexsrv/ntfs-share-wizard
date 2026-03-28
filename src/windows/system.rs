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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsAdminStatus {
    pub is_elevated: bool,
    pub summary: String,
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

pub fn admin_status() -> WindowsAdminStatus {
    admin_status_impl()
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

#[cfg(windows)]
fn admin_status_impl() -> WindowsAdminStatus {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[bool](([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let is_elevated = String::from_utf8_lossy(&output.stdout)
                .trim()
                .eq_ignore_ascii_case("true");

            WindowsAdminStatus {
                is_elevated,
                summary: if is_elevated {
                    "O terminal atual esta rodando com privilegios administrativos.".to_owned()
                } else {
                    "O terminal atual nao esta elevado. Abra o app como administrador para alterar o Fast Startup.".to_owned()
                },
            }
        }
        Ok(output) => WindowsAdminStatus {
            is_elevated: false,
            summary: format!(
                "Nao foi possivel confirmar privilegios administrativos. Saida: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        },
        Err(error) => WindowsAdminStatus {
            is_elevated: false,
            summary: format!("Nao foi possivel verificar privilegios administrativos: {error}"),
        },
    }
}

#[cfg(not(windows))]
fn disable_fast_startup_impl() -> Result<CommandReport> {
    bail!("powercfg /h off is only available on Windows")
}

#[cfg(not(windows))]
fn admin_status_impl() -> WindowsAdminStatus {
    WindowsAdminStatus {
        is_elevated: false,
        summary: "A verificacao de privilegios administrativos esta disponivel apenas no Windows."
            .to_owned(),
    }
}
