use serde::{Deserialize, Serialize};

use crate::app::App;
use crate::linux::fstab;
use crate::linux::mount;
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
                "instalado".to_owned()
            } else {
                format!(
                    "ausente\nPlano de instalacao:\n{}",
                    format_install_plan(state.install_plan())
                )
            }
        })
        .unwrap_or_else(|| "ainda nao verificado".to_owned());

    format!(
        "[INFO] Fluxo detectado: {}.\nLinux distro: {}\nntfs-3g: {}\nSystem module: {}\nTarget mount point: {}\nDefault SteamLibrary: {}\nSelected partition: {}\n\nPressione Enter para continuar no wizard Linux.",
        app.operating_system().display_name(),
        system_info.distro.display_name(),
        ntfs_3g_summary,
        system_info.platform_label,
        system_info.fstab_mount_point,
        system::default_steam_library_path(),
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
    MountValidation,
    MountCreateConfirm,
    MountCreateResult,
    FstabReview,
    FstabWriteConfirm,
    FstabWriteResult,
    MountApplyConfirm,
    MountApplyResult,
    SteamLibraryCreateConfirm,
    SteamLibraryCreateResult,
    FinalGuidance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxWizardState {
    current_screen: LinuxScreen,
    distro: system::LinuxDistro,
    partitions: Vec<system::NtfsPartition>,
    selected_index: usize,
    selected_partition: Option<system::NtfsPartition>,
    message: Option<String>,
    ntfs_3g_installed: bool,
    install_plan: system::InstallPlan,
    install_report: Option<system::InstallExecutionReport>,
    mount_layout: system::MountLayoutStatus,
    path_creation_report: Option<system::PathCreationReport>,
    fstab_write_report: Option<fstab::FstabWriteReport>,
    mount_apply_report: Option<mount::MountApplyReport>,
    steam_library_create_report: Option<mount::SteamLibraryCreateReport>,
}

impl LinuxWizardState {
    pub fn new() -> Self {
        let distro = system::LinuxDistro::Unknown;
        Self {
            current_screen: LinuxScreen::InstallPlan,
            distro: distro.clone(),
            partitions: Vec::new(),
            selected_index: 0,
            selected_partition: None,
            message: None,
            ntfs_3g_installed: false,
            install_plan: system::install_plan_for_distro(&distro),
            install_report: None,
            mount_layout: system::validate_mount_layout(),
            path_creation_report: None,
            fstab_write_report: None,
            mount_apply_report: None,
            steam_library_create_report: None,
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

    pub fn selected_partition(&self) -> Option<&system::NtfsPartition> {
        self.selected_partition.as_ref()
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

    pub fn mount_layout(&self) -> &system::MountLayoutStatus {
        &self.mount_layout
    }

    pub fn path_creation_report(&self) -> Option<&system::PathCreationReport> {
        self.path_creation_report.as_ref()
    }

    pub fn fstab_write_report(&self) -> Option<&fstab::FstabWriteReport> {
        self.fstab_write_report.as_ref()
    }

    pub fn mount_apply_report(&self) -> Option<&mount::MountApplyReport> {
        self.mount_apply_report.as_ref()
    }

    pub fn steam_library_create_report(&self) -> Option<&mount::SteamLibraryCreateReport> {
        self.steam_library_create_report.as_ref()
    }
}

pub fn load_partitions(state: &mut LinuxWizardState) {
    state.distro = system::detect_distro();
    state.ntfs_3g_installed = system::is_ntfs_3g_installed();
    state.install_plan = system::install_plan_for_distro(state.distro());
    state.install_report = None;
    state.mount_layout = system::validate_mount_layout();
    state.path_creation_report = None;
    state.selected_partition = None;
    state.fstab_write_report = None;
    state.mount_apply_report = None;
    state.steam_library_create_report = None;

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
            if state.ntfs_3g_installed() {
                refresh_partition_state(state);
            } else if state.install_plan().execution_mode == system::InstallExecutionMode::Assisted
            {
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
            if state.ntfs_3g_installed() {
                refresh_partition_state(state);
            }
            None
        }
        LinuxScreen::PartitionSelection => {
            if let Some(partition) = state.partitions.get(state.selected_index).cloned() {
                state.selected_partition = Some(partition.clone());
                state.mount_layout = system::validate_mount_layout();
                state.path_creation_report = None;
                state.fstab_write_report = None;
                state.mount_apply_report = None;
                state.steam_library_create_report = None;
                state.current_screen = LinuxScreen::MountValidation;
                Some(partition)
            } else {
                None
            }
        }
        LinuxScreen::NoPartitions => None,
        LinuxScreen::MountValidation => {
            if needs_creation(state.mount_layout()) && can_offer_creation(state.mount_layout()) {
                state.current_screen = LinuxScreen::MountCreateConfirm;
            } else if mount_layout_ready(state.mount_layout()) {
                state.current_screen = LinuxScreen::FstabReview;
            }
            None
        }
        LinuxScreen::MountCreateConfirm => {
            state.path_creation_report = Some(system::create_missing_mount_layout());
            state.mount_layout = state
                .path_creation_report()
                .map(|report| report.mount_layout.clone())
                .unwrap_or_else(system::validate_mount_layout);
            state.current_screen = LinuxScreen::MountCreateResult;
            None
        }
        LinuxScreen::MountCreateResult => {
            state.mount_layout = system::validate_mount_layout();
            if mount_layout_ready(state.mount_layout()) {
                state.current_screen = LinuxScreen::FstabReview;
            } else {
                state.current_screen = LinuxScreen::MountValidation;
            }
            None
        }
        LinuxScreen::FstabReview => {
            state.current_screen = LinuxScreen::FstabWriteConfirm;
            None
        }
        LinuxScreen::FstabWriteConfirm => {
            if let Some(partition) = state.selected_partition() {
                state.fstab_write_report = Some(fstab::write_entry(partition));
            }
            state.current_screen = LinuxScreen::FstabWriteResult;
            None
        }
        LinuxScreen::FstabWriteResult => {
            if state
                .fstab_write_report()
                .map(|report| report.success)
                .unwrap_or(false)
            {
                state.current_screen = LinuxScreen::MountApplyConfirm;
            }
            None
        }
        LinuxScreen::MountApplyConfirm => {
            state.mount_apply_report = Some(mount::apply_mount_and_validate());
            state.current_screen = LinuxScreen::MountApplyResult;
            None
        }
        LinuxScreen::MountApplyResult => {
            if let Some(report) = state.mount_apply_report() {
                if report.success && !report.steam_library_exists && report.steam_library_can_create
                {
                    state.current_screen = LinuxScreen::SteamLibraryCreateConfirm;
                } else if report.success {
                    state.current_screen = LinuxScreen::FinalGuidance;
                }
            }
            None
        }
        LinuxScreen::SteamLibraryCreateConfirm => {
            state.steam_library_create_report = Some(mount::create_steam_library_directory());
            state.current_screen = LinuxScreen::SteamLibraryCreateResult;
            None
        }
        LinuxScreen::SteamLibraryCreateResult => {
            if state
                .steam_library_create_report()
                .map(|report| report.success && report.steam_library_exists)
                .unwrap_or(false)
            {
                state.current_screen = LinuxScreen::FinalGuidance;
            }
            None
        }
        LinuxScreen::FinalGuidance => None,
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
        LinuxScreen::MountValidation => {
            state.current_screen = LinuxScreen::PartitionSelection;
            false
        }
        LinuxScreen::MountCreateConfirm => {
            state.current_screen = LinuxScreen::MountValidation;
            false
        }
        LinuxScreen::MountCreateResult => {
            state.current_screen = LinuxScreen::MountValidation;
            false
        }
        LinuxScreen::FstabReview => {
            state.current_screen = LinuxScreen::MountValidation;
            false
        }
        LinuxScreen::FstabWriteConfirm => {
            state.current_screen = LinuxScreen::FstabReview;
            false
        }
        LinuxScreen::FstabWriteResult => {
            state.current_screen = LinuxScreen::FstabReview;
            false
        }
        LinuxScreen::MountApplyConfirm => {
            state.current_screen = LinuxScreen::FstabWriteResult;
            false
        }
        LinuxScreen::MountApplyResult => {
            state.current_screen = LinuxScreen::MountApplyConfirm;
            false
        }
        LinuxScreen::SteamLibraryCreateConfirm => {
            state.current_screen = LinuxScreen::MountApplyResult;
            false
        }
        LinuxScreen::SteamLibraryCreateResult => {
            state.current_screen = LinuxScreen::MountApplyResult;
            false
        }
        LinuxScreen::FinalGuidance => {
            if state.steam_library_create_report().is_some() {
                state.current_screen = LinuxScreen::SteamLibraryCreateResult;
            } else {
                state.current_screen = LinuxScreen::MountApplyResult;
            }
            false
        }
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
            body: "[ERROR] O estado do wizard Linux nao esta disponivel.".to_owned(),
        };
    };

    match state.current_screen() {
        LinuxScreen::InstallPlan => install_plan_view(state),
        LinuxScreen::InstallConfirm => install_confirm_view(state),
        LinuxScreen::InstallResult => install_result_view(state),
        LinuxScreen::PartitionSelection => partition_selection_view(state),
        LinuxScreen::NoPartitions => no_partitions_view(state),
        LinuxScreen::MountValidation => mount_validation_view(state),
        LinuxScreen::MountCreateConfirm => mount_create_confirm_view(state),
        LinuxScreen::MountCreateResult => mount_create_result_view(state),
        LinuxScreen::FstabReview => fstab_review_view(state),
        LinuxScreen::FstabWriteConfirm => fstab_write_confirm_view(state),
        LinuxScreen::FstabWriteResult => fstab_write_result_view(state),
        LinuxScreen::MountApplyConfirm => mount_apply_confirm_view(state),
        LinuxScreen::MountApplyResult => mount_apply_result_view(state),
        LinuxScreen::SteamLibraryCreateConfirm => steam_library_create_confirm_view(state),
        LinuxScreen::SteamLibraryCreateResult => steam_library_create_result_view(state),
        LinuxScreen::FinalGuidance => final_guidance_view(state),
    }
}

pub fn key_hints(state: Option<&LinuxWizardState>) -> &'static str {
    match state.map(LinuxWizardState::current_screen) {
        Some(LinuxScreen::InstallPlan) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::InstallConfirm) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::InstallResult) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::PartitionSelection) => {
            "Up/Down mover | Enter confirmar | Esc voltar | q sair"
        }
        Some(LinuxScreen::NoPartitions) => "Esc voltar | q sair",
        Some(LinuxScreen::MountValidation) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::MountCreateConfirm) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::MountCreateResult) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::FstabReview) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::FstabWriteConfirm) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::FstabWriteResult) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::MountApplyConfirm) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::MountApplyResult) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::SteamLibraryCreateConfirm) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::SteamLibraryCreateResult) => "Enter confirmar | Esc voltar | q sair",
        Some(LinuxScreen::FinalGuidance) => "Esc voltar | q sair",
        None => "q sair",
    }
}

fn install_plan_view(state: &LinuxWizardState) -> View<'static> {
    let assisted_note = match state.install_plan().execution_mode {
        system::InstallExecutionMode::Assisted => {
            "[INFO] Esta distro suporta execucao assistida. Pressione Enter para revisar a confirmacao."
        }
        system::InstallExecutionMode::GuidedOnly => {
            "[WARNING] Esta distro usa fluxo apenas guiado. Revise os passos e execute manualmente fora do wizard."
        }
    };

    View {
        title: "Linux | Instalar ntfs-3g",
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
        title: "Linux | Confirmar Instalacao",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\n[INFO] O wizard esta pronto para executar estes comandos passo a passo:\n\n{}\n\n[INFO] Cada comando captura stdout/stderr e interrompe em caso de falha.\nLoading: a execucao comeca logo apos a confirmacao.",
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
            "Nenhum comando foi executado.".to_owned()
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
                        .unwrap_or_else(|| "desconhecido".to_owned()),
                    present_output(&result.stdout),
                    present_output(&result.stderr)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        None => "Nenhum relatorio de instalacao esta disponivel.".to_owned(),
    };
    let summary = report
        .map(|value| value.summary.as_str())
        .unwrap_or("O fluxo de instalacao ainda nao foi executado.");
    let next_step = if state.ntfs_3g_installed() {
        "[INFO] Pressione Enter para continuar para a deteccao de particoes NTFS."
    } else {
        "[WARNING] O ntfs-3g continua ausente, entao o wizard nao avancara para particoes nem para o fstab."
    };

    View {
        title: "Linux | Resultado da Instalacao",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\n{}\n\n{}\n\n{}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            status_line(state.ntfs_3g_installed(), summary),
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
        title: "Linux | Selecionar Particao NTFS",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\n[INFO] Particoes NTFS detectadas, ordenadas da maior para a menor:\n\n{}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            partitions
        ),
    }
}

fn no_partitions_view(state: &LinuxWizardState) -> View<'static> {
    View {
        title: "Linux | Nenhuma Particao NTFS",
        body: format!(
            "Linux distro: {}\nntfs-3g: {}\n\n[WARNING] {}",
            state.distro().display_name(),
            ntfs_3g_status_summary(state),
            state
                .message()
                .unwrap_or("Nenhuma particao NTFS elegivel foi encontrada.")
        ),
    }
}

fn mount_validation_view(state: &LinuxWizardState) -> View<'static> {
    let guidance = if needs_creation(state.mount_layout())
        && can_offer_creation(state.mount_layout())
    {
        "Pressione Enter para revisar a criacao das pastas ausentes."
    } else if needs_creation(state.mount_layout()) {
        "Alguns caminhos estao ausentes, mas nao podem ser criados com seguranca por este wizard. Revise os detalhes."
    } else {
        "Os caminhos padrao do mountpoint e da SteamLibrary ja existem como diretorios reais. Pressione Enter para revisar a entrada gerada do fstab."
    };

    View {
        title: "Linux | Validar Caminhos",
        body: format!(
            "{}\n\n[INFO] {}",
            format_mount_layout(state.mount_layout()),
            guidance
        ),
    }
}

fn mount_create_confirm_view(state: &LinuxWizardState) -> View<'static> {
    View {
        title: "Linux | Confirmar Criacao de Pastas",
        body: format!(
            "{}\n\n[INFO] O wizard criara diretorios reais quando necessario.\n[WARNING] Symlinks nao sao usados neste fluxo.\n\nLoading: a criacao sera iniciada logo apos a confirmacao.",
            format_mount_layout(state.mount_layout())
        ),
    }
}

fn mount_create_result_view(state: &LinuxWizardState) -> View<'static> {
    let report = state.path_creation_report();
    let summary = report
        .map(|value| value.summary.as_str())
        .unwrap_or("No folder creation report is available.");

    View {
        title: "Linux | Resultado da Criacao",
        body: format!(
            "{}\n\n{}\n\n[INFO] Pressione Enter para continuar.",
            status_line(!summary.to_ascii_lowercase().contains("failed"), summary),
            format_mount_layout(state.mount_layout())
        ),
    }
}

fn fstab_review_view(state: &LinuxWizardState) -> View<'static> {
    let Some(partition) = state.selected_partition() else {
        return View {
            title: "Review fstab Entry",
            body: "[ERROR] Nenhuma particao NTFS foi selecionada para gerar a entrada do fstab."
                .to_owned(),
        };
    };

    View {
        title: "Linux | Revisar Entrada do fstab",
        body: format!(
            "Selected partition: {}\nSize: {}\nUUID: {}\nMountpoint: {}\n\n[INFO] Linha exata que sera gravada:\n{}",
            partition.path,
            system::human_readable_size(partition.size_bytes),
            partition.uuid,
            system::default_mountpoint(),
            system::generate_fstab_entry(partition)
        ),
    }
}

fn fstab_write_confirm_view(state: &LinuxWizardState) -> View<'static> {
    let Some(partition) = state.selected_partition() else {
        return View {
            title: "Confirm fstab Write",
            body:
                "[ERROR] Nenhuma particao NTFS foi selecionada para a escrita segura em /etc/fstab."
                    .to_owned(),
        };
    };

    View {
        title: "Linux | Confirmar Escrita no fstab",
        body: format!(
            "[INFO] O wizard vai:\n1. Criar um backup timestampado de /etc/fstab\n2. Garantir que {} exista\n3. Ignorar a escrita se UUID={} ja estiver presente\n4. Acrescentar a nova linha ao final de /etc/fstab\n\n[INFO] Linha a ser gravada:\n{}\n\nLoading: a escrita segura comeca logo apos a confirmacao.",
            system::default_mountpoint(),
            partition.uuid,
            system::generate_fstab_entry(partition)
        ),
    }
}

fn fstab_write_result_view(state: &LinuxWizardState) -> View<'static> {
    let Some(report) = state.fstab_write_report() else {
        return View {
            title: "fstab Write Result",
            body: "[ERROR] Nenhum resultado de escrita em /etc/fstab esta disponivel ainda."
                .to_owned(),
        };
    };

    View {
        title: "Linux | Resultado da Escrita no fstab",
        body: format!(
            "{}\nBackup created: {}\nEntry already existed: {}\n\nLine:\n{}",
            status_line(report.success, &report.summary),
            report.backup_path.as_deref().unwrap_or("<none>"),
            if report.entry_already_exists {
                "yes"
            } else {
                "no"
            },
            report.written_line
        ),
    }
}

fn mount_apply_confirm_view(_state: &LinuxWizardState) -> View<'static> {
    View {
        title: "Linux | Aplicar Montagem",
        body: "[INFO] O wizard executara `mount -a`, verificara se /media/gamedisk foi montado, testara escrita e validara /media/gamedisk/SteamLibrary.\n\nLoading: a verificacao da montagem comeca logo apos a confirmacao.".to_owned(),
    }
}

fn mount_apply_result_view(state: &LinuxWizardState) -> View<'static> {
    let Some(report) = state.mount_apply_report() else {
        return View {
            title: "Linux | Resultado da Montagem",
            body: "[ERROR] Nenhum resultado de validacao de montagem esta disponivel ainda."
                .to_owned(),
        };
    };

    let readonly = report
        .readonly_diagnostic
        .as_deref()
        .unwrap_or("No readonly diagnostic.");
    let fast_startup = report
        .fast_startup_warning
        .as_deref()
        .unwrap_or("No Fast Startup warning detected.");
    let next_step = if report.success
        && !report.steam_library_exists
        && report.steam_library_can_create
    {
        "[WARNING] A SteamLibrary esta ausente. Pressione Enter para revisar a criacao da pasta."
    } else if report.success {
        "[SUCCESS] A validacao da montagem foi concluida com sucesso."
    } else {
        "[WARNING] Revise os diagnosticos abaixo antes de tentar novamente. Se a particao parecer insegura, verifique o Fast Startup no Windows."
    };

    View {
        title: "Linux | Resultado da Montagem",
        body: format!(
            "{}\nMounted: {}\nRead-write: {}\nWrite test: {}\nSteamLibrary exists: {}\n\nmount -a stdout:\n{}\n\nmount -a stderr:\n{}\n\nReadonly diagnostic:\n{}\n\nFast Startup warning:\n{}\n\n{}",
            status_line(report.success, &report.summary),
            yes_no(report.mountpoint_mounted),
            yes_no(report.read_write),
            yes_no(report.write_test_succeeded),
            yes_no(report.steam_library_exists),
            present_output(&report.mount_command_stdout),
            present_output(&report.mount_command_stderr),
            readonly,
            fast_startup,
            next_step
        ),
    }
}

fn steam_library_create_confirm_view(_state: &LinuxWizardState) -> View<'static> {
    View {
        title: "Linux | Criar SteamLibrary",
        body: format!(
            "[INFO] A particao esta montada e com escrita liberada, mas {} ainda nao existe.\n\nLoading: a criacao da pasta comeca logo apos a confirmacao.",
            system::default_steam_library_path()
        ),
    }
}

fn steam_library_create_result_view(state: &LinuxWizardState) -> View<'static> {
    let Some(report) = state.steam_library_create_report() else {
        return View {
            title: "SteamLibrary Result",
            body: "[ERROR] Nenhum resultado da criacao da SteamLibrary esta disponivel ainda."
                .to_owned(),
        };
    };

    View {
        title: "Linux | Resultado da SteamLibrary",
        body: format!(
            "{}\nSteamLibrary exists: {}\n\n[INFO] Pressione Enter para continuar para as orientacoes finais.",
            status_line(report.success, &report.summary),
            yes_no(report.steam_library_exists)
        ),
    }
}

fn final_guidance_view(state: &LinuxWizardState) -> View<'static> {
    let partition_summary = state
        .selected_partition()
        .map(|partition| {
            format!(
                "Partition: {}\nUUID: {}\nSize: {}",
                partition.path,
                partition.uuid,
                system::human_readable_size(partition.size_bytes)
            )
        })
        .unwrap_or_else(|| "Partition: <none>\nUUID: <none>\nSize: <desconhecido>".to_owned());
    let mount_summary = state
        .mount_apply_report()
        .map(|report| {
            let mut lines = vec![
                format!("Mounted: {}", yes_no(report.mountpoint_mounted)),
                format!("Read-write: {}", yes_no(report.read_write)),
                format!("Write test: {}", yes_no(report.write_test_succeeded)),
                format!(
                    "SteamLibrary exists: {}",
                    yes_no(report.steam_library_exists)
                ),
            ];

            if let Some(readonly) = &report.readonly_diagnostic {
                lines.push(format!("Readonly diagnostic: {readonly}"));
            }

            if let Some(fast_startup) = &report.fast_startup_warning {
                lines.push(format!("Fast Startup warning: {fast_startup}"));
            }

            lines.join("\n")
        })
        .unwrap_or_else(|| {
            "Mounted: no\nRead-write: no\nWrite test: no\nSteamLibrary exists: no".to_owned()
        });
    let windows_warning = match state.mount_apply_report() {
        Some(report) if !report.read_write || report.fast_startup_warning.is_some() => {
            "[WARNING] Correcao no Windows:\nSe o volume NTFS parecer somente leitura ou inseguro, inicialize no Windows, desabilite o Fast Startup, rode `powercfg /h off` se necessario e depois faca um desligamento completo com `shutdown /s /t 0` antes de tentar novamente no Linux.".to_owned()
        }
        _ => "[INFO] Orientacao para Windows:\nUse a mesma pasta de biblioteca Steam nos dois sistemas e mantenha o disco sempre desligado corretamente antes de alternar entre Windows e Linux.".to_owned(),
    };

    View {
        title: "Linux | Compartilhamento Final",
        body: format!(
            "Distro: {}\n{}\nMountpoint: {}\nSteam library path: {}\nntfs-3g: {}\n\nMount status:\n{}\n\n[INFO] Passos finais:\n1. No Steam do Linux, adicione ou use a biblioteca em {}\n2. No Steam do Windows, aponte a biblioteca para a mesma pasta\n3. [WARNING] Nao use symlinks neste setup\n4. Mantenha os dois sistemas apontando para exatamente o mesmo diretorio\n\n[SUCCESS] Destino compartilhado:\n{}\n\n{}",
            state.distro().display_name(),
            partition_summary,
            system::default_mountpoint(),
            system::default_steam_library_path(),
            ntfs_3g_status_summary(state),
            mount_summary,
            system::default_steam_library_path(),
            system::default_steam_library_path(),
            windows_warning
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

fn format_mount_layout(layout: &system::MountLayoutStatus) -> String {
    format!(
        "Mountpoint base: {}\n  status: {}\n\nSteam library path: {}\n  status: {}",
        layout.mountpoint.path,
        describe_path_validation(&layout.mountpoint),
        layout.steam_library.path,
        describe_path_validation(&layout.steam_library)
    )
}

fn describe_path_validation(path: &system::PathValidation) -> String {
    if path.is_symlink {
        "existe como symlink (correcao manual necessaria)".to_owned()
    } else if path.exists && path.is_directory {
        "existe como diretorio".to_owned()
    } else if path.exists {
        "existe, mas nao e um diretorio".to_owned()
    } else if path.can_create {
        "ausente, pode ser criado".to_owned()
    } else {
        "ausente, o caminho pai nao esta pronto".to_owned()
    }
}

fn needs_creation(layout: &system::MountLayoutStatus) -> bool {
    !layout.mountpoint.exists || !layout.steam_library.exists
}

fn mount_layout_ready(layout: &system::MountLayoutStatus) -> bool {
    layout.mountpoint.exists
        && layout.mountpoint.is_directory
        && !layout.mountpoint.is_symlink
        && layout.steam_library.exists
        && layout.steam_library.is_directory
        && !layout.steam_library.is_symlink
}

fn can_offer_creation(layout: &system::MountLayoutStatus) -> bool {
    !layout.mountpoint.is_symlink
        && !layout.steam_library.is_symlink
        && (!layout.mountpoint.exists || layout.mountpoint.is_directory)
        && (!layout.steam_library.exists || layout.steam_library.is_directory)
}

fn present_output(value: &str) -> &str {
    if value.is_empty() { "<empty>" } else { value }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn status_line(success: bool, message: &str) -> String {
    if success {
        format!("[SUCCESS] {message}")
    } else {
        format!("[ERROR] {message}")
    }
}
