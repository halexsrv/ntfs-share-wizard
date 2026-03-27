use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsSystemInfo {
    pub platform_label: &'static str,
}

pub fn inspect() -> WindowsSystemInfo {
    WindowsSystemInfo {
        platform_label: "windows",
    }
}
