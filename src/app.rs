use serde::{Deserialize, Serialize};

use crate::linux::system::NtfsPartition;
use crate::linux::wizard::LinuxWizardState;
use crate::os::OperatingSystem;
use crate::windows::wizard::WindowsWizardState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Welcome,
    DetectedSystem,
    LinuxWizard,
    Unsupported,
    WindowsWizard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    operating_system: OperatingSystem,
    current_screen: Screen,
    should_quit: bool,
    linux_wizard: Option<LinuxWizardState>,
    selected_linux_partition: Option<NtfsPartition>,
    windows_wizard: Option<WindowsWizardState>,
}

impl App {
    pub fn new(operating_system: OperatingSystem) -> Self {
        Self {
            current_screen: Screen::Welcome,
            linux_wizard: matches!(operating_system, OperatingSystem::Linux(_))
                .then(LinuxWizardState::new),
            selected_linux_partition: None,
            windows_wizard: matches!(operating_system, OperatingSystem::Windows)
                .then(WindowsWizardState::new),
            operating_system,
            should_quit: false,
        }
    }

    pub fn operating_system(&self) -> &OperatingSystem {
        &self.operating_system
    }

    pub fn current_screen(&self) -> Screen {
        self.current_screen
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn linux_wizard(&self) -> Option<&LinuxWizardState> {
        self.linux_wizard.as_ref()
    }

    pub fn selected_linux_partition(&self) -> Option<&NtfsPartition> {
        self.selected_linux_partition.as_ref()
    }

    pub fn windows_wizard(&self) -> Option<&WindowsWizardState> {
        self.windows_wizard.as_ref()
    }

    pub fn advance(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Welcome => {
                if self.operating_system.is_supported() {
                    Screen::DetectedSystem
                } else {
                    Screen::Unsupported
                }
            }
            Screen::DetectedSystem => match self.operating_system {
                OperatingSystem::Windows => Screen::WindowsWizard,
                OperatingSystem::Linux(_) => {
                    if let Some(wizard) = self.linux_wizard.as_mut() {
                        crate::linux::wizard::load_partitions(wizard);
                    }
                    Screen::LinuxWizard
                }
                OperatingSystem::Unsupported(_) => self.current_screen,
            },
            Screen::LinuxWizard => {
                if let Some(wizard) = self.linux_wizard.as_mut() {
                    if let Some(partition) = crate::linux::wizard::advance(wizard) {
                        self.selected_linux_partition = Some(partition);
                    }
                }
                self.current_screen
            }
            Screen::WindowsWizard => {
                if let Some(wizard) = self.windows_wizard.as_mut() {
                    crate::windows::wizard::advance(wizard);
                }
                self.current_screen
            }
            Screen::Unsupported => self.current_screen,
        };
    }

    pub fn go_back(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Welcome => Screen::Welcome,
            Screen::DetectedSystem | Screen::Unsupported => Screen::Welcome,
            Screen::LinuxWizard => {
                if let Some(wizard) = self.linux_wizard.as_mut() {
                    if crate::linux::wizard::go_back(wizard) {
                        Screen::DetectedSystem
                    } else {
                        Screen::LinuxWizard
                    }
                } else {
                    Screen::DetectedSystem
                }
            }
            Screen::WindowsWizard => {
                if let Some(wizard) = self.windows_wizard.as_mut() {
                    if crate::windows::wizard::go_back(wizard) {
                        Screen::DetectedSystem
                    } else {
                        Screen::WindowsWizard
                    }
                } else {
                    Screen::DetectedSystem
                }
            }
        };
    }

    pub fn request_quit(&mut self) {
        self.should_quit = true;
    }

    pub fn move_selection_up(&mut self) {
        if self.current_screen == Screen::LinuxWizard {
            if let Some(wizard) = self.linux_wizard.as_mut() {
                crate::linux::wizard::move_selection_up(wizard);
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.current_screen == Screen::LinuxWizard {
            if let Some(wizard) = self.linux_wizard.as_mut() {
                crate::linux::wizard::move_selection_down(wizard);
            }
        }
    }
}
