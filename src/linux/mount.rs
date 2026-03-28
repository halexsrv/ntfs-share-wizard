use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io;
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
use crate::linux::system;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountApplyReport {
    pub success: bool,
    pub mount_command_stdout: String,
    pub mount_command_stderr: String,
    pub mountpoint_mounted: bool,
    pub read_write: bool,
    pub write_test_succeeded: bool,
    pub readonly_diagnostic: Option<String>,
    pub fast_startup_warning: Option<String>,
    pub steam_library_exists: bool,
    pub steam_library_can_create: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteamLibraryCreateReport {
    pub success: bool,
    pub summary: String,
    pub steam_library_exists: bool,
}

pub fn apply_mount_and_validate() -> MountApplyReport {
    apply_mount_and_validate_impl()
}

pub fn create_steam_library_directory() -> SteamLibraryCreateReport {
    create_steam_library_directory_impl()
}

#[cfg(target_os = "linux")]
fn apply_mount_and_validate_impl() -> MountApplyReport {
    let privilege = system::privilege_status();
    if !privilege.is_root {
        return MountApplyReport {
            success: false,
            mount_command_stdout: String::new(),
            mount_command_stderr: String::new(),
            mountpoint_mounted: false,
            read_write: false,
            write_test_succeeded: false,
            readonly_diagnostic: None,
            fast_startup_warning: None,
            steam_library_exists: false,
            steam_library_can_create: false,
            summary: format!(
                "A aplicacao de `mount -a` e a validacao final exigem privilegios de root. {}",
                privilege.summary
            ),
        };
    }

    let output = Command::new("mount").arg("-a").output();
    let (stdout, stderr) = match output {
        Ok(output) => (
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ),
        Err(error) => {
            return MountApplyReport {
                success: false,
                mount_command_stdout: String::new(),
                mount_command_stderr: friendly_io_error(&error),
                mountpoint_mounted: false,
                read_write: false,
                write_test_succeeded: false,
                readonly_diagnostic: None,
                fast_startup_warning: None,
                steam_library_exists: false,
                steam_library_can_create: false,
                summary: format!(
                    "Nao foi possivel executar `mount -a`: {}",
                    friendly_io_error(&error)
                ),
            };
        }
    };

    let mount_status = inspect_mountpoint();
    let write_test = if mount_status.mounted && !mount_status.readonly {
        try_write_test()
    } else {
        WriteTestResult {
            succeeded: false,
            diagnostic: None,
        }
    };
    let steam_library_status = validate_steam_library_path();
    let readonly_diagnostic = if mount_status.mounted && mount_status.readonly {
        Some("O mountpoint esta montado, mas atualmente esta em modo somente leitura.".to_owned())
    } else {
        write_test.diagnostic.clone()
    };
    let combined_output = format!("{stdout}\n{stderr}");
    let fast_startup_warning =
        detect_fast_startup_warning(&combined_output, readonly_diagnostic.as_deref());
    let success = mount_status.mounted && write_test.succeeded;

    MountApplyReport {
        success,
        mount_command_stdout: stdout,
        mount_command_stderr: stderr,
        mountpoint_mounted: mount_status.mounted,
        read_write: mount_status.mounted && !mount_status.readonly && write_test.succeeded,
        write_test_succeeded: write_test.succeeded,
        readonly_diagnostic,
        fast_startup_warning,
        steam_library_exists: steam_library_status.exists
            && steam_library_status.is_directory
            && !steam_library_status.is_symlink,
        steam_library_can_create: !steam_library_status.exists && steam_library_status.can_create,
        summary: if success {
            "O `mount -a` funcionou, o mountpoint esta com escrita liberada e a particao esta pronta para validacao."
                .to_owned()
        } else if !mount_status.mounted {
            "O `mount -a` nao deixou `/media/gamedisk` montado. Revise a saida do comando e a entrada do fstab."
                .to_owned()
        } else if mount_status.readonly {
            "A particao esta montada, mas apenas em modo somente leitura.".to_owned()
        } else {
            "O mountpoint foi encontrado, mas o teste de escrita falhou.".to_owned()
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_mount_and_validate_impl() -> MountApplyReport {
    MountApplyReport {
        success: false,
        mount_command_stdout: String::new(),
        mount_command_stderr: String::new(),
        mountpoint_mounted: false,
        read_write: false,
        write_test_succeeded: false,
        readonly_diagnostic: None,
        fast_startup_warning: None,
        steam_library_exists: false,
        steam_library_can_create: false,
        summary: "A validacao de montagem esta disponivel apenas no Linux.".to_owned(),
    }
}

#[cfg(target_os = "linux")]
fn create_steam_library_directory_impl() -> SteamLibraryCreateReport {
    let privilege = system::privilege_status();
    if !privilege.is_root {
        return SteamLibraryCreateReport {
            success: false,
            summary: format!(
                "A criacao da SteamLibrary exige privilegios de root. {}",
                privilege.summary
            ),
            steam_library_exists: false,
        };
    }

    let validation = system::validate_mount_layout().steam_library;
    if validation.is_symlink {
        return SteamLibraryCreateReport {
            success: false,
            summary: "A SteamLibrary existe como symlink, e o wizard nao substitui symlinks."
                .to_owned(),
            steam_library_exists: false,
        };
    }

    if validation.exists && !validation.is_directory {
        return SteamLibraryCreateReport {
            success: false,
            summary: "A SteamLibrary existe, mas nao e um diretorio.".to_owned(),
            steam_library_exists: false,
        };
    }

    match fs::create_dir_all(system::default_steam_library_path()) {
        Ok(()) => SteamLibraryCreateReport {
            success: true,
            summary: "O diretorio SteamLibrary esta pronto.".to_owned(),
            steam_library_exists: true,
        },
        Err(error) => SteamLibraryCreateReport {
            success: false,
            summary: format!(
                "Nao foi possivel criar o diretorio SteamLibrary: {}",
                friendly_io_error(&error)
            ),
            steam_library_exists: false,
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn create_steam_library_directory_impl() -> SteamLibraryCreateReport {
    SteamLibraryCreateReport {
        success: false,
        summary: "A criacao da SteamLibrary esta disponivel apenas no Linux.".to_owned(),
        steam_library_exists: false,
    }
}

#[cfg(target_os = "linux")]
struct MountInspectResult {
    mounted: bool,
    readonly: bool,
}

#[cfg(target_os = "linux")]
fn inspect_mountpoint() -> MountInspectResult {
    let mountinfo = fs::read_to_string("/proc/mounts").unwrap_or_default();
    for line in mountinfo.lines() {
        let mut parts = line.split_whitespace();
        let _source = parts.next();
        let mountpoint = parts.next();
        let _fstype = parts.next();
        let options = parts.next();

        if mountpoint == Some(system::default_mountpoint()) {
            let readonly = options
                .unwrap_or_default()
                .split(',')
                .any(|option| option == "ro");
            return MountInspectResult {
                mounted: true,
                readonly,
            };
        }
    }

    MountInspectResult {
        mounted: false,
        readonly: false,
    }
}

#[cfg(target_os = "linux")]
struct WriteTestResult {
    succeeded: bool,
    diagnostic: Option<String>,
}

#[cfg(target_os = "linux")]
fn try_write_test() -> WriteTestResult {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let test_path = format!(
        "{}/.ntfs-share-wizard-write-test-{timestamp}",
        system::default_mountpoint()
    );

    match fs::write(&test_path, b"write-test") {
        Ok(()) => {
            let _ = fs::remove_file(&test_path);
            WriteTestResult {
                succeeded: true,
                diagnostic: None,
            }
        }
        Err(error) => WriteTestResult {
            succeeded: false,
            diagnostic: Some(format!(
                "O teste de escrita falhou: {}",
                friendly_io_error(&error)
            )),
        },
    }
}

#[cfg(target_os = "linux")]
fn validate_steam_library_path() -> system::PathValidation {
    system::validate_mount_layout().steam_library
}

#[cfg(target_os = "linux")]
fn detect_fast_startup_warning(output: &str, diagnostic: Option<&str>) -> Option<String> {
    let combined = format!(
        "{} {}",
        output.to_ascii_lowercase(),
        diagnostic.unwrap_or_default().to_ascii_lowercase()
    );
    let suspicious = [
        "hibernat",
        "unsafe state",
        "fast startup",
        "windows is hibernated",
        "metadata kept in windows cache",
        "volume is dirty",
    ];

    suspicious.iter().any(|needle| combined.contains(needle)).then(|| {
        "A particao NTFS pode estar em um estado inseguro vindo do Windows. Verifique se o Fast Startup ainda esta ativo e faca um desligamento completo do Windows antes de tentar novamente.".to_owned()
    })
}

#[cfg(target_os = "linux")]
fn friendly_io_error(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::PermissionDenied {
        "permissao negada. Execute o app novamente com privilegios suficientes.".to_owned()
    } else {
        error.to_string()
    }
}
