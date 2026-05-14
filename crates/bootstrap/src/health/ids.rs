//! String form of ComponentId / PackageManagerId — needed because the
//! type-erased Component.id field is a String (mixes the two enums).

use super::types::{ComponentId, PackageManagerId, Platform};

pub fn component_id_str(id: ComponentId) -> &'static str {
    match id {
        ComponentId::Postgres => "postgres",
        ComponentId::Ollama => "ollama",
        ComponentId::Sensei => "sensei",
        ComponentId::Database => "database",
        ComponentId::Daemon => "daemon",
    }
}

pub fn package_manager_id_str(id: PackageManagerId) -> &'static str {
    match id {
        PackageManagerId::Homebrew => "homebrew",
        PackageManagerId::Winget => "winget",
    }
}

pub fn package_manager_for_platform(p: Platform) -> PackageManagerId {
    match p {
        Platform::Windows => PackageManagerId::Winget,
        _ => PackageManagerId::Homebrew,
    }
}

pub fn parse_component_id(s: &str) -> Option<ComponentId> {
    match s {
        "postgres" => Some(ComponentId::Postgres),
        "ollama" => Some(ComponentId::Ollama),
        "sensei" => Some(ComponentId::Sensei),
        "database" => Some(ComponentId::Database),
        "daemon" => Some(ComponentId::Daemon),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_component_id_round_trip() {
        for id in [
            ComponentId::Postgres,
            ComponentId::Ollama,
            ComponentId::Sensei,
            ComponentId::Database,
            ComponentId::Daemon,
        ] {
            assert_eq!(parse_component_id(component_id_str(id)), Some(id));
        }
        assert_eq!(parse_component_id("nope"), None);
    }

    #[test]
    fn package_manager_for_platform_returns_winget_on_windows() {
        assert_eq!(
            package_manager_for_platform(Platform::Windows),
            PackageManagerId::Winget
        );
        assert_eq!(
            package_manager_for_platform(Platform::Macos),
            PackageManagerId::Homebrew
        );
        assert_eq!(
            package_manager_for_platform(Platform::Linux),
            PackageManagerId::Homebrew
        );
    }

    #[test]
    fn package_manager_id_str_for_each_variant() {
        assert_eq!(package_manager_id_str(PackageManagerId::Homebrew), "homebrew");
        assert_eq!(package_manager_id_str(PackageManagerId::Winget), "winget");
    }
}
