use serde::{Deserialize, Serialize};

pub mod detect;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatingSystem {
    Windows,
    Linux,
    Unsupported(String),
}

impl OperatingSystem {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::Unsupported(name) => name.as_str(),
        }
    }

    pub fn is_supported(&self) -> bool {
        matches!(self, Self::Windows | Self::Linux)
    }
}
