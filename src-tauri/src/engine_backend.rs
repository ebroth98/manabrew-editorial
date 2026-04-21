pub mod java_backend;
pub mod rust_backend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineBackendKind {
    Rust,
    JavaForge,
}

impl EngineBackendKind {
    pub fn from_env() -> Self {
        std::env::var("OPEN_MAGIC_ENGINE_BACKEND")
            .ok()
            .and_then(|value| Self::parse(&value))
            .unwrap_or(Self::Rust)
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "rust" | "rust-engine" => Some(Self::Rust),
            "java" | "java-forge" | "forge-java" => Some(Self::JavaForge),
            _ => None,
        }
    }

    pub fn is_supported(self) -> bool {
        match self {
            Self::Rust => true,
            Self::JavaForge => cfg!(feature = "java-forge"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::JavaForge => "java-forge",
        }
    }
}
