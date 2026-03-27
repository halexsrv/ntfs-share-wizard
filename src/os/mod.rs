pub mod detect;

#[derive(Debug, Clone)]
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
}
