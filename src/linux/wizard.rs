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

    format!(
        "Detected {} flow.\nLinux distro: {}\nSystem module: {}\nTarget mount point: {}\nSelected partition: {}\nPress Enter to open the NTFS partition selector.",
        app.operating_system().display_name(),
        system_info.distro.display_name(),
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
    partitions: Vec<system::NtfsPartition>,
    selected_index: usize,
    message: Option<String>,
}

impl LinuxWizardState {
    pub fn new() -> Self {
        Self {
            current_screen: LinuxScreen::PartitionSelection,
            partitions: Vec::new(),
            selected_index: 0,
            message: None,
        }
    }

    pub fn current_screen(&self) -> LinuxScreen {
        self.current_screen
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
}

pub fn load_partitions(state: &mut LinuxWizardState) {
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
    let body = state
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
        body,
    }
}

fn no_partitions_view(state: &LinuxWizardState) -> View<'static> {
    View {
        title: "No NTFS Partitions",
        body: state
            .message()
            .unwrap_or("Nenhuma particao NTFS elegivel foi encontrada.")
            .to_owned(),
    }
}
