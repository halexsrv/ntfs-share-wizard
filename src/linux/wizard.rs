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
        "Detected {} flow.\nLinux distro: {}\nntfs-3g: {}\nSystem module: {}\nTarget mount point: {}\nSelected partition: {}\nPress Enter to open the NTFS partition selector.",
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
}

impl LinuxWizardState {
    pub fn new() -> Self {
        let distro = system::LinuxDistro::Unknown;
        Self {
            current_screen: LinuxScreen::PartitionSelection,
            distro: distro.clone(),
            partitions: Vec::new(),
            selected_index: 0,
            message: None,
            ntfs_3g_installed: false,
            install_plan: system::install_plan_for_distro(&distro),
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
}

pub fn load_partitions(state: &mut LinuxWizardState) {
    state.distro = system::detect_distro();
    state.ntfs_3g_installed = system::is_ntfs_3g_installed();
    state.install_plan = system::install_plan_for_distro(state.distro());

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

pub fn advance(state: &mut LinuxWizardState) -> Option<system::NtfsPartition> {
    match state.current_screen {
        LinuxScreen::PartitionSelection => state.partitions.get(state.selected_index).cloned(),
        LinuxScreen::NoPartitions => None,
    }
}

pub fn go_back(state: &mut LinuxWizardState) -> bool {
    match state.current_screen {
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
            title: "Linux Partitions",
            body: "Linux wizard state is unavailable.".to_owned(),
        };
    };

    match state.current_screen() {
        LinuxScreen::PartitionSelection => partition_selection_view(state),
        LinuxScreen::NoPartitions => no_partitions_view(state),
    }
}

pub fn key_hints(state: Option<&LinuxWizardState>) -> &'static str {
    match state.map(LinuxWizardState::current_screen) {
        Some(LinuxScreen::PartitionSelection) => {
            "Up/Down: move | Enter: select | Esc: back | q: quit"
        }
        Some(LinuxScreen::NoPartitions) => "Esc: back | q: quit",
        None => "q: quit",
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

    let body = format!(
        "Linux distro: {}\nntfs-3g: {}\n\n{}\n\nDetected NTFS partitions:\n\n{}",
        state.distro().display_name(),
        ntfs_3g_status_summary(state),
        format_install_plan(state.install_plan()),
        partitions
    );

    View {
        title: "NTFS Partitions",
        body,
    }
}

fn no_partitions_view(state: &LinuxWizardState) -> View<'static> {
    View {
        title: "No NTFS Partitions",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\n{}\n\n{}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            format_install_plan(state.install_plan()),
            state
                .message()
                .unwrap_or("Nenhuma particao NTFS elegivel foi encontrada.")
        ),
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
            .map(|(index, step)| format!("{}. {}", index + 1, step))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    sections.join("\n")
}
