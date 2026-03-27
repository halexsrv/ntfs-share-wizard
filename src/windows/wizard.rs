use serde::{Deserialize, Serialize};

use crate::app::App;
use crate::tui::View;
use crate::windows::system;

pub fn detected_system_details(app: &App) -> String {
    let system_info = system::inspect();
    format!(
        "Detected {} flow.\nSystem module: {}\nPress Enter to open the Fast Startup wizard.",
        app.operating_system().display_name(),
        system_info.platform_label
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowsScreen {
    Explanation,
    Confirmation,
    Execution,
    Result,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsWizardState {
    current_screen: WindowsScreen,
    last_result: Option<WindowsCommandResult>,
}

impl WindowsWizardState {
    pub fn new() -> Self {
        Self {
            current_screen: WindowsScreen::Explanation,
            last_result: None,
        }
    }

    pub fn current_screen(&self) -> WindowsScreen {
        self.current_screen
    }

    pub fn last_result(&self) -> Option<&WindowsCommandResult> {
        self.last_result.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsCommandResult {
    pub success: bool,
    pub summary: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

pub fn advance(state: &mut WindowsWizardState) {
    match state.current_screen {
        WindowsScreen::Explanation => state.current_screen = WindowsScreen::Confirmation,
        WindowsScreen::Confirmation => state.current_screen = WindowsScreen::Execution,
        WindowsScreen::Execution => {
            state.last_result = Some(run_disable_fast_startup());
            state.current_screen = WindowsScreen::Result;
        }
        WindowsScreen::Result => {}
    }
}

pub fn go_back(state: &mut WindowsWizardState) -> bool {
    match state.current_screen {
        WindowsScreen::Explanation => true,
        WindowsScreen::Confirmation => {
            state.current_screen = WindowsScreen::Explanation;
            false
        }
        WindowsScreen::Execution => {
            state.current_screen = WindowsScreen::Confirmation;
            false
        }
        WindowsScreen::Result => {
            state.current_screen = WindowsScreen::Execution;
            false
        }
    }
}

pub fn current_view(app: &App) -> View<'static> {
    let Some(state) = app.windows_wizard() else {
        return View {
            title: "Windows Wizard",
            body: "Windows wizard state is unavailable.".to_owned(),
        };
    };

    match state.current_screen() {
        WindowsScreen::Explanation => View {
            title: "Fast Startup",
            body: [
                "Fast Startup combines elements of hibernation and shutdown to speed up boot times.",
                "",
                "For NTFS sharing workflows, this can leave the Windows volume in a hybrid state and make safe access from Linux less reliable.",
                "",
                "Disabling Fast Startup helps ensure the NTFS volume is fully closed before you mount it elsewhere.",
            ]
            .join("\n"),
        },
        WindowsScreen::Confirmation => View {
            title: "Confirm Change",
            body: [
                "The wizard is ready to disable Fast Startup by running:",
                "powercfg /h off",
                "",
                "This may require administrative privileges. If the command fails, reopen the app in an elevated terminal and try again.",
                "",
                "Press Enter to continue to the execution step.",
            ]
            .join("\n"),
        },
        WindowsScreen::Execution => View {
            title: "Execute Command",
            body: [
                "Press Enter to execute:",
                "powercfg /h off",
                "",
                "This step runs only in the Windows flow.",
                "Administrative privileges may be required.",
            ]
            .join("\n"),
        },
        WindowsScreen::Result => result_view(state),
    }
}

pub fn key_hints(state: Option<&WindowsWizardState>) -> &'static str {
    match state.map(WindowsWizardState::current_screen) {
        Some(WindowsScreen::Explanation) => "Enter: next | Esc: back | q: quit",
        Some(WindowsScreen::Confirmation) => "Enter: next | Esc: back | q: quit",
        Some(WindowsScreen::Execution) => "Enter: run command | Esc: back | q: quit",
        Some(WindowsScreen::Result) => "Esc: back | q: quit",
        None => "q: quit",
    }
}

fn result_view(state: &WindowsWizardState) -> View<'static> {
    let result = state.last_result();
    let body = match result {
        Some(result) => {
            let stdout = present_output("stdout", &result.stdout);
            let stderr = present_output("stderr", &result.stderr);
            format!(
                "{}\n\n{}\n{}\nExit code: {}\n\nRecommended next step:\nshutdown /s /t 0",
                result.summary,
                stdout,
                stderr,
                result
                    .exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            )
        }
        None => [
            "No command result is available yet.",
            "",
            "Recommended next step after a successful change:",
            "shutdown /s /t 0",
        ]
        .join("\n"),
    };

    View {
        title: "Result",
        body,
    }
}

fn run_disable_fast_startup() -> WindowsCommandResult {
    match system::disable_fast_startup() {
        Ok(report) if report.success => WindowsCommandResult {
            success: true,
            summary: "Fast Startup was disabled successfully.".to_owned(),
            stdout: report.stdout,
            stderr: report.stderr,
            exit_code: report.exit_code,
        },
        Ok(report) => WindowsCommandResult {
            success: false,
            summary: "Windows reported a failure while disabling Fast Startup. This often means the terminal is not elevated.".to_owned(),
            stdout: report.stdout,
            stderr: report.stderr,
            exit_code: report.exit_code,
        },
        Err(error) => WindowsCommandResult {
            success: false,
            summary: format!("Could not execute powercfg /h off: {error}"),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
        },
    }
}

fn present_output(label: &str, value: &str) -> String {
    if value.is_empty() {
        format!("{label}: <empty>")
    } else {
        format!("{label}: {value}")
    }
}
