use crate::os::OperatingSystem;

#[derive(Debug, Clone)]
pub struct App {
    operating_system: OperatingSystem,
}

impl App {
    pub fn new(operating_system: OperatingSystem) -> Self {
        Self { operating_system }
    }

    pub fn operating_system(&self) -> &OperatingSystem {
        &self.operating_system
    }
}
