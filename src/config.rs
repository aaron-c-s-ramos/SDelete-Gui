use std::path::PathBuf;
use thiserror::Error;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

/// Defines the minimum permitted overwrite pass count.
pub const MIN_PASSES: u8 = 1;

/// Defines the maximum permitted overwrite pass count.
pub const MAX_PASSES: u8 = 100;

/// Defines the default overwrite pass count.
pub const DEFAULT_PASSES: u8 = 10;

/// Defines quick pass-count presets for the Explorer cascading submenu.
pub const SUBMENU_PASS_PRESETS: [u8; 5] = [1, 3, 5, 7, 35];

const SETTINGS_SUBKEY: &str = r"Software\SDelete-Gui";

/// Errors produced when configuration values are invalid or unreadable.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Pass count {0} is outside the valid range ({MIN_PASSES}..={MAX_PASSES}).")]
    InvalidPassCount(u8),
    #[error("Registry error: {0}")]
    Registry(#[from] std::io::Error),
}

/// Validated pass count guaranteed to stay inside 1..=100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PassCount(u8);

impl PassCount {
    /// Creates a validated pass count.
    pub fn new(value: u8) -> Result<Self, ConfigError> {
        if (MIN_PASSES..=MAX_PASSES).contains(&value) {
            Ok(Self(value))
        } else {
            Err(ConfigError::InvalidPassCount(value))
        }
    }

    /// Clamps any integer into the valid pass-count range.
    pub fn clamped(value: u32) -> Self {
        let clamped = value.clamp(u32::from(MIN_PASSES), u32::from(MAX_PASSES)) as u8;
        Self(clamped)
    }

    /// Returns the numeric pass count.
    pub fn get(self) -> u8 {
        self.0
    }
}

impl Default for PassCount {
    fn default() -> Self {
        Self(DEFAULT_PASSES)
    }
}

/// Identifies the operation that Sysinternals SDelete executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SDeleteMode {
    Delete,
    CleanFreeSpace,
    ZeroFreeSpace,
}

impl SDeleteMode {
    /// Returns the stable CLI token for this mode.
    pub const fn as_cli_token(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::CleanFreeSpace => "clean-free-space",
            Self::ZeroFreeSpace => "zero-free-space",
        }
    }

    /// Parses a CLI token into a mode.
    pub fn parse_cli_token(token: &str) -> Option<Self> {
        if token.eq_ignore_ascii_case("delete") {
            Some(Self::Delete)
        } else if token.eq_ignore_ascii_case("clean-free-space") {
            Some(Self::CleanFreeSpace)
        } else if token.eq_ignore_ascii_case("zero-free-space") {
            Some(Self::ZeroFreeSpace)
        } else {
            None
        }
    }
}

/// Selects the Windows Registry scope for context-menu registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryScope {
    /// Registers under HKCU\Software\Classes without administrator elevation.
    CurrentUser,
    /// Registers under HKLM\Software\Classes for all users (requires elevation).
    AllUsers,
}

impl RegistryScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentUser => "current-user",
            Self::AllUsers => "all-users",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("all-users") {
            Self::AllUsers
        } else {
            Self::CurrentUser
        }
    }
}

/// Selects dark or light visual styling in the desktop GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("light") {
            Self::Light
        } else {
            Self::Dark
        }
    }
}

/// Selects the display language for labels, prompts, and context menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Spanish,
    German,
}

impl Language {
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
            Self::German => "de",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("es") || value.to_ascii_lowercase().starts_with("es-") {
            Self::Spanish
        } else if value.eq_ignore_ascii_case("de") || value.to_ascii_lowercase().starts_with("de-")
        {
            Self::German
        } else {
            Self::English
        }
    }
}

/// Stores all persisted SDelete-Gui settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SDeleteConfig {
    pub passes: PassCount,
    pub confirm_before_delete: bool,
    pub sequential_execution: bool,
    pub use_pass_submenu: bool,
    pub show_context_menu_icon: bool,
    pub enable_audit_log: bool,
    pub log_file_path: PathBuf,
    pub registry_scope: RegistryScope,
    pub theme: ThemeMode,
    pub language: Language,
}

impl Default for SDeleteConfig {
    fn default() -> Self {
        Self {
            passes: PassCount::default(),
            confirm_before_delete: true,
            sequential_execution: true,
            use_pass_submenu: true,
            show_context_menu_icon: true,
            enable_audit_log: false,
            log_file_path: default_log_file_path(),
            registry_scope: RegistryScope::CurrentUser,
            theme: ThemeMode::Dark,
            language: Language::English,
        }
    }
}

/// Computes the default per-user audit log file path.
pub fn default_log_file_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("SDelete-Gui").join("sdelete-audit.log")
}

impl SDeleteConfig {
    /// Replaces an empty log path with the default location.
    pub fn normalize(&mut self) {
        if self.log_file_path.as_os_str().is_empty() {
            self.log_file_path = default_log_file_path();
        }
    }

    /// Loads persisted configuration from HKCU\Software\SDelete-Gui.
    pub fn load() -> Self {
        let mut cfg = Self::default();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(key) = hkcu.open_subkey_with_flags(SETTINGS_SUBKEY, KEY_READ) else {
            return cfg;
        };

        if let Ok(raw_passes) = key.get_value::<u32, _>("NumberOfPasses") {
            cfg.passes = PassCount::clamped(raw_passes);
        }
        if let Ok(v) = key.get_value::<u32, _>("ConfirmBeforeDelete") {
            cfg.confirm_before_delete = v != 0;
        }
        if let Ok(v) = key.get_value::<u32, _>("SequentialExecution") {
            cfg.sequential_execution = v != 0;
        }
        if let Ok(v) = key.get_value::<u32, _>("UsePassSubmenu") {
            cfg.use_pass_submenu = v != 0;
        }
        if let Ok(v) = key.get_value::<u32, _>("ShowContextMenuIcon") {
            cfg.show_context_menu_icon = v != 0;
        }
        if let Ok(v) = key.get_value::<u32, _>("EnableAuditLog") {
            cfg.enable_audit_log = v != 0;
        }
        if let Ok(path) = key.get_value::<String, _>("LogFilePath") {
            if !path.trim().is_empty() {
                cfg.log_file_path = PathBuf::from(path);
            }
        }
        if let Ok(scope) = key.get_value::<String, _>("RegistryScope") {
            cfg.registry_scope = RegistryScope::parse(&scope);
        }
        if let Ok(theme) = key.get_value::<String, _>("ThemeMode") {
            cfg.theme = ThemeMode::parse(&theme);
        }
        if let Ok(lang) = key.get_value::<String, _>("Language") {
            cfg.language = Language::parse(&lang);
        }

        cfg.normalize();
        cfg
    }

    /// Saves configuration to HKCU\Software\SDelete-Gui.
    pub fn save(&mut self) -> Result<(), ConfigError> {
        self.normalize();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey_with_flags(SETTINGS_SUBKEY, KEY_WRITE)?;

        key.set_value("NumberOfPasses", &u32::from(self.passes.get()))?;
        key.set_value(
            "ConfirmBeforeDelete",
            &u32::from(self.confirm_before_delete),
        )?;
        key.set_value("SequentialExecution", &u32::from(self.sequential_execution))?;
        key.set_value("UsePassSubmenu", &u32::from(self.use_pass_submenu))?;
        key.set_value(
            "ShowContextMenuIcon",
            &u32::from(self.show_context_menu_icon),
        )?;
        key.set_value("EnableAuditLog", &u32::from(self.enable_audit_log))?;
        key.set_value(
            "LogFilePath",
            &self.log_file_path.to_string_lossy().as_ref(),
        )?;
        key.set_value("RegistryScope", &self.registry_scope.as_str())?;
        key.set_value("ThemeMode", &self.theme.as_str())?;
        key.set_value("Language", &self.language.as_code())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_count_rejects_out_of_range_values() {
        assert!(PassCount::new(0).is_err());
        assert!(PassCount::new(101).is_err());
        assert_eq!(PassCount::new(1).unwrap().get(), 1);
        assert_eq!(PassCount::new(100).unwrap().get(), 100);
    }

    #[test]
    fn pass_count_clamps_bounds() {
        assert_eq!(PassCount::clamped(0).get(), 1);
        assert_eq!(PassCount::clamped(500).get(), 100);
        assert_eq!(PassCount::clamped(35).get(), 35);
    }

    #[test]
    fn mode_cli_tokens_round_trip() {
        for mode in [
            SDeleteMode::Delete,
            SDeleteMode::CleanFreeSpace,
            SDeleteMode::ZeroFreeSpace,
        ] {
            let token = mode.as_cli_token();
            assert_eq!(SDeleteMode::parse_cli_token(token), Some(mode));
        }
        assert_eq!(SDeleteMode::parse_cli_token("unknown"), None);
    }
}
