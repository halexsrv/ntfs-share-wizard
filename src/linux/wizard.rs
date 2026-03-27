use serde::{Deserialize, Serialize};

use crate::app::App;
use crate::linux::system;
use crate::os::OperatingSystem;
use crate::tui::View;

pub fn detected_system_details(app: &App) -> String {
    let distro = match app.operating_system() {
        OperatingSystem::Linux(distro) => distro.clone(),
        _ => system::LinuxDistro::Unknown,
    };
    let system_info = system::inspect(distro);
    let selected_partition = app
        .selected_linux_partition()
        .map(system::friendly_partition_title)
        .unwrap_or_else(|| "Nenhuma particao NTFS selecionada ainda.".to_owned());
    let ntfs_3g_summary = app
        .linux_wizard()
        .map(|state| {
            if state.ntfs_3g_installed() {
                "installed".to_owned()
            } else {
                format!(
                    "missing\nInstall plan:\n{}",
                    format_install_plan(state.install_plan())
                )
            }
        })
        .unwrap_or_else(|| "not checked yet".to_owned());

    format!(
        "Detected {} flow.\nLinux distro: {}\nntfs-3g: {}\nSystem module: {}\nTarget mount point: {}\nSelected partition: {}\nPress Enter to continue in the Linux wizard.",
        app.operating_system().display_name(),
        system_info.distro.display_name(),
        ntfs_3g_summary,
        system_info.platform_label,
        system_info.fstab_mount_point,
        selected_partition
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinuxScreen {
    InstallPlan,
    InstallConfirm,
    InstallResult,
    PartitionSelection,
    NoPartitions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxWizardState {
    current_screen: LinuxScreen,
    distro: system::LinuxDistro,
    partitions: Vec<system::NtfsPartition>,
    selected_index: usize,
    message: Option<String>,
    ntfs_3g_installed: bool,
    install_plan: system::InstallPlan,
    install_report: Option<system::InstallExecutionReport>,
}

impl LinuxWizardState {
    pub fn new() -> Self {
        let distro = system::LinuxDistro::Unknown;
        Self {
            current_screen: LinuxScreen::InstallPlan,
            distro: distro.clone(),
            partitions: Vec::new(),
            selected_index: 0,
            message: None,
            ntfs_3g_installed: false,
            install_plan: system::install_plan_for_distro(&distro),
            install_report: None,
        }
    }

    pub fn current_screen(&self) -> LinuxScreen {
        self.current_screen
    }

    pub fn distro(&self) -> &system::LinuxDistro {
        &self.distro
    }

    pub fn partitions(&self) -> &[system::NtfsPartition] {
        &self.partitions
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn ntfs_3g_installed(&self) -> bool {
        self.ntfs_3g_installed
    }

    pub fn install_plan(&self) -> &system::InstallPlan {
        &self.install_plan
    }

    pub fn install_report(&self) -> Option<&system::InstallExecutionReport> {
        self.install_report.as_ref()
    }
}

pub fn load_partitions(state: &mut LinuxWizardState) {
    state.distro = system::detect_distro();
    state.ntfs_3g_installed = system::is_ntfs_3g_installed();
    state.install_plan = system::install_plan_for_distro(state.distro());
    state.install_report = None;

    if state.ntfs_3g_installed {
        refresh_partition_state(state);
    } else {
        state.current_screen = LinuxScreen::InstallPlan;
        state.partitions.clear();
        state.selected_index = 0;
        state.message = Some("O ntfs-3g precisa estar instalado antes de prosseguir.".to_owned());
    }
}

pub fn advance(state: &mut LinuxWizardState) -> Option<system::NtfsPartition> {
    match state.current_screen {
        LinuxScreen::InstallPlan => {
            if state.ntfs_3g_installed {
                refresh_partition_state(state);
            } else if state.install_plan.execution_mode == system::InstallExecutionMode::Assisted {
                state.current_screen = LinuxScreen::InstallConfirm;
            }
            None
        }
        LinuxScreen::InstallConfirm => {
            state.install_report = Some(system::execute_install_plan(state.distro()));
            state.ntfs_3g_installed = system::is_ntfs_3g_installed();
            state.current_screen = LinuxScreen::InstallResult;
            None
        }
        LinuxScreen::InstallResult => {
            if state.ntfs_3g_installed {
                refresh_partition_state(state);
            }
            None
        }
        LinuxScreen::PartitionSelection => state.partitions.get(state.selected_index).cloned(),
        LinuxScreen::NoPartitions => None,
    }
}

pub fn go_back(state: &mut LinuxWizardState) -> bool {
    match state.current_screen {
        LinuxScreen::InstallPlan => true,
        LinuxScreen::InstallConfirm => {
            state.current_screen = LinuxScreen::InstallPlan;
            false
        }
        LinuxScreen::InstallResult => {
            state.current_screen = LinuxScreen::InstallPlan;
            false
        }
        LinuxScreen::PartitionSelection | LinuxScreen::NoPartitions => true,
    }
}

pub fn move_selection_up(state: &mut LinuxWizardState) {
    if state.current_screen == LinuxScreen::PartitionSelection && state.selected_index > 0 {
        state.selected_index -= 1;
    }
}

pub fn move_selection_down(state: &mut LinuxWizardState) {
    if state.current_screen == LinuxScreen::PartitionSelection
        && state.selected_index + 1 < state.partitions.len()
    {
        state.selected_index += 1;
    }
}

pub fn current_view(app: &App) -> View<'static> {
    let Some(state) = app.linux_wizard() else {
        return View {
            title: "Linux Wizard",
            body: "Linux wizard state is unavailable.".to_owned(),
        };
    };

    match state.current_screen() {
        LinuxScreen::InstallPlan => install_plan_view(state),
        LinuxScreen::InstallConfirm => install_confirm_view(state),
        LinuxScreen::InstallResult => install_result_view(state),
        LinuxScreen::PartitionSelection => partition_selection_view(state),
        LinuxScreen::NoPartitions => no_partitions_view(state),
    }
}

pub fn key_hints(state: Option<&LinuxWizardState>) -> &'static str {
    match state.map(LinuxWizardState::current_screen) {
        Some(LinuxScreen::InstallPlan) => "Enter: continue | Esc: back | q: quit",
        Some(LinuxScreen::InstallConfirm) => "Enter: run install plan | Esc: back | q: quit",
        Some(LinuxScreen::InstallResult) => {
            "Enter: continue when ntfs-3g is installed | Esc: back | q: quit"
        }
        Some(LinuxScreen::PartitionSelection) => {
            "Up/Down: move | Enter: select | Esc: back | q: quit"
        }
        Some(LinuxScreen::NoPartitions) => "Esc: back | q: quit",
        None => "q: quit",
    }
}

fn install_plan_view(state: &LinuxWizardState) -> View<'static> {
    let assisted_note = match state.install_plan().execution_mode {
        system::InstallExecutionMode::Assisted => {
            "This distro supports assisted execution. Press Enter to review the confirmation screen."
        }
        system::InstallExecutionMode::GuidedOnly => {
            "This distro uses a guided-only flow. Review the steps carefully and run them manually outside the wizard."
        }
    };

    View {
        title: "Install ntfs-3g",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\n{}\n\n{}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            format_install_plan(state.install_plan()),
            assisted_note
        ),
    }
}

fn install_confirm_view(state: &LinuxWizardState) -> View<'static> {
    View {
        title: "Confirm Install",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\nThe wizard is ready to execute these commands step by step:\n\n{}\n\nEach command will capture stdout/stderr and stop on failure.",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            format_install_plan(state.install_plan())
        ),
    }
}

fn install_result_view(state: &LinuxWizardState) -> View<'static> {
    let report = state.install_report();
    let results = match report {
        Some(report) if report.command_results.is_empty() => {
            "No commands were executed.".to_owned()
        }
        Some(report) => report
            .command_results
            .iter()
            .map(|result| {
                format!(
                    "{}\n  command: {}\n  success: {}\n  exit code: {}\n  stdout: {}\n  stderr: {}",
                    result.label,
                    result.command.as_deref().unwrap_or("<none>"),
                    if result.success { "yes" } else { "no" },
                    result
                        .exit_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "unknown".to_owned()),
                    present_output(&result.stdout),
                    present_output(&result.stderr)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        None => "No installation report is available.".to_owned(),
    };
    let summary = report
        .map(|value| value.summary.as_str())
        .unwrap_or("The installation flow has not been executed yet.");
    let next_step = if state.ntfs_3g_installed() {
        "Press Enter to continue to NTFS partition detection."
    } else {
        "ntfs-3g is still unavailable, so the wizard will not continue to partition or fstab steps."
    };

    View {
        title: "Install Result",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\nSummary: {}\n\n{}\n\n{}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            summary,
            results,
            next_step
        ),
    }
}

fn partition_selection_view(state: &LinuxWizardState) -> View<'static> {
    let partitions = state
        .partitions()
        .iter()
        .enumerate()
        .map(|(index, partition)| {
            let cursor = if index == state.selected_index() {
                ">"
            } else {
                " "
            };
            let label = partition
                .label
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("Sem label");
            let mountpoint = partition.mountpoint.as_deref().unwrap_or("-");

            format!(
                "{cursor} {}\n  path: {}\n  size: {}\n  label: {}\n  uuid: {}\n  mountpoint: {}",
                system::friendly_partition_title(partition),
                partition.path,
                system::human_readable_size(partition.size_bytes),
                label,
                partition.uuid,
                mountpoint
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    View {
        title: "NTFS Partitions",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\nDetected NTFS partitions:\n\n{}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            partitions
        ),
    }
}

fn no_partitions_view(state: &LinuxWizardState) -> View<'static> {
    View {
        title: "No NTFS Partitions",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\n{}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            state
                .message()
                .unwrap_or("Nenhuma particao NTFS elegivel foi encontrada.")
        ),
    }
}

fn refresh_partition_state(state: &mut LinuxWizardState) {
    match system::detect_ntfs_partitions() {
        Ok(partitions) if partitions.is_empty() => {
            state.partitions.clear();
            state.selected_index = 0;
            state.current_screen = LinuxScreen::NoPartitions;
            state.message = Some(
                "Nenhuma particao NTFS elegivel foi encontrada neste sistema Linux.".to_owned(),
            );
        }
        Ok(partitions) => {
            state.partitions = partitions;
            state.selected_index = 0;
            state.current_screen = LinuxScreen::PartitionSelection;
            state.message = None;
        }
        Err(error) => {
            state.partitions.clear();
            state.selected_index = 0;
            state.current_screen = LinuxScreen::NoPartitions;
            state.message = Some(format!("Nao foi possivel detectar particoes NTFS: {error}"));
        }
    }
}

fn ntfs_3g_status_summary(state: &LinuxWizardState) -> &'static str {
    if state.ntfs_3g_installed() {
        "installed"
    } else {
        "missing"
    }
}

fn format_install_plan(plan: &system::InstallPlan) -> String {
    let mut sections = vec![plan.title.clone()];

    if let Some(caution) = &plan.caution {
        sections.push(format!("Caution: {caution}"));
    }

    sections.push(
        plan.steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let command = step
                    .command_preview
                    .as_deref()
                    .unwrap_or("(manual guidance)");
                format!("{}. {}\n   {}", index + 1, step.label, command)
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );

    sections.join("\n")
}

fn present_output(value: &str) -> &str {
    if value.is_empty() { "<empty>" } else { value }
}
