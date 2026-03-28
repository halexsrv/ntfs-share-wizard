use serde::{Deserialize, Serialize};

use crate::app::App;
use crate::tui::View;
use crate::windows::system;

pub fn detected_system_details(app: &App) -> String {
    let system_info = system::inspect();
    let admin = system::admin_status();
    format!(
        "[INFO] Fluxo detectado: {}.\nSystem module: {}\nAdmin: {}\n\nPressione Enter para abrir o assistente de Fast Startup.",
        app.operating_system().display_name(),
        system_info.platform_label,
        admin.summary
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
            body: "[ERROR] O estado do wizard Windows nao esta disponivel.".to_owned(),
        };
    };

    match state.current_screen() {
        WindowsScreen::Explanation => View {
            title: "Windows | Fast Startup",
            body: [
                "[INFO] Fast Startup combina hibernacao parcial com desligamento para acelerar o boot.",
                "",
                "[WARNING] Em compartilhamento de particao NTFS entre Windows e Linux, isso pode deixar o volume em estado hibrido.",
                "",
                "[SUCCESS] Desabilitar o Fast Startup ajuda a garantir que o NTFS seja fechado corretamente antes de montar no Linux.",
            ]
            .join("\n"),
        },
        WindowsScreen::Confirmation => View {
            title: "Windows | Confirmar Alteracao",
            body: {
                let admin = system::admin_status();
                vec![
                    "[INFO] O wizard esta pronto para desabilitar o Fast Startup com o comando:"
                        .to_owned(),
                    "powercfg /h off".to_owned(),
                    String::new(),
                    "[WARNING] Isso pode exigir privilegios administrativos. Se falhar, abra o app em um terminal elevado e tente novamente.".to_owned(),
                    String::new(),
                    format!("Admin: {}", admin.summary),
                    String::new(),
                    "Pressione Enter para confirmar.".to_owned(),
                ]
                .join("\n")
            },
        },
        WindowsScreen::Execution => View {
            title: "Windows | Executar Comando",
            body: {
                let admin = system::admin_status();
                vec![
                    "[INFO] Tela de execucao pronta.".to_owned(),
                    "Pressione Enter para executar:".to_owned(),
                    "powercfg /h off".to_owned(),
                    String::new(),
                    "[INFO] Esta etapa roda apenas no fluxo Windows.".to_owned(),
                    "[WARNING] Privilegios administrativos podem ser necessarios.".to_owned(),
                    format!("Admin: {}", admin.summary),
                    String::new(),
                    "Loading: o comando sera executado logo apos a confirmacao.".to_owned(),
                ]
                .join("\n")
            },
        },
        WindowsScreen::Result => result_view(state),
    }
}

pub fn key_hints(state: Option<&WindowsWizardState>) -> &'static str {
    match state.map(WindowsWizardState::current_screen) {
        Some(WindowsScreen::Explanation) => "Enter confirmar | Esc voltar | q sair",
        Some(WindowsScreen::Confirmation) => "Enter confirmar | Esc voltar | q sair",
        Some(WindowsScreen::Execution) => "Enter confirmar | Esc voltar | q sair",
        Some(WindowsScreen::Result) => "Esc voltar | q sair",
        None => "q sair",
    }
}

fn result_view(state: &WindowsWizardState) -> View<'static> {
    let result = state.last_result();
    let body = match result {
        Some(result) => {
            let stdout = present_output("stdout", &result.stdout);
            let stderr = present_output("stderr", &result.stderr);
            format!(
                "{}\n\n{}\n{}\nExit code: {}\n\n[INFO] Proximo passo recomendado:\nshutdown /s /t 0",
                status_tag(result.success, &result.summary),
                stdout,
                stderr,
                result
                    .exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "desconhecido".to_owned())
            )
        }
        None => [
            "[WARNING] Nenhum resultado de comando esta disponivel ainda.",
            "",
            "[INFO] Proximo passo recomendado apos uma execucao bem-sucedida:",
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
    let admin = system::admin_status();
    if !admin.is_elevated {
        return WindowsCommandResult {
            success: false,
            summary: admin.summary,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
        };
    }

    match system::disable_fast_startup() {
        Ok(report) if report.success => WindowsCommandResult {
            success: true,
            summary: "O Fast Startup foi desabilitado com sucesso.".to_owned(),
            stdout: report.stdout,
            stderr: report.stderr,
            exit_code: report.exit_code,
        },
        Ok(report) => WindowsCommandResult {
            success: false,
            summary: "O Windows reportou falha ao desabilitar o Fast Startup. Em muitos casos isso significa que o terminal nao esta elevado.".to_owned(),
            stdout: report.stdout,
            stderr: report.stderr,
            exit_code: report.exit_code,
        },
        Err(error) => WindowsCommandResult {
            success: false,
            summary: format!("Nao foi possivel executar `powercfg /h off`: {error}"),
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

fn status_tag(success: bool, message: &str) -> String {
    if success {
        format!("[SUCCESS] {message}")
    } else {
        format!("[ERROR] {message}")
    }
}
