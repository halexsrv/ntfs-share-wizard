use serde::{Deserialize, Serialize};

use crate::os::OperatingSystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Welcome,
    DetectedSystem,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    operating_system: OperatingSystem,
    current_screen: Screen,
    should_quit: bool,
}

impl App {
    pub fn new(operating_system: OperatingSystem) -> Self {
        Self {
            current_screen: Screen::Welcome,
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

    pub fn advance(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Welcome => {
                if self.operating_system.is_supported() {
                    Screen::DetectedSystem
                } else {
                    Screen::Unsupported
                }
            }
            Screen::DetectedSystem | Screen::Unsupported => self.current_screen,
        };
    }

    pub fn go_back(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Welcome => Screen::Welcome,
            Screen::DetectedSystem | Screen::Unsupported => Screen::Welcome,
        };
    }

    pub fn request_quit(&mut self) {
        self.should_quit = true;
    }
}
