use std::io;
use std::path::Path;

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_WRITE};
use winreg::RegKey;

use crate::command::build_shell_runner_command;
use crate::config::{PassCount, RegistryScope, SDeleteConfig, SDeleteMode, SUBMENU_PASS_PRESETS};
use crate::i18n::{format_default_pass_label, format_pass_label, strings_for};

/// Registry key name used by SDelete-Gui under each Explorer shell root.
pub const ROOT_SHELL_KEY_NAME: &str = "SDeleteGui";

/// Standard Windows yellow warning triangle icon resource for destructive context actions.
pub const DEFAULT_WARNING_ICON: &str = "imageres.dll,-84";

const TARGET_ROOTS: [&str; 3] = [r"*\shell", r"Directory\shell", r"Drive\shell"];
const LEGACY_KEY_NAMES: [&str; 4] = [
    "Secure Delete",
    "Borrado Seguro",
    "Sicher Löschen",
    "Sicher Loeschen",
];

/// Represents one Explorer context-menu entry or cascading submenu tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuSpec {
    pub shell_root: String,
    pub key_name: String,
    pub mui_verb: String,
    pub icon: Option<String>,
    pub command: Option<String>,
    pub sub_items: Vec<ContextMenuSpec>,
}

/// Builds all Explorer context-menu specifications for files, directories, and drives.
pub fn build_menu_specs(runner_exe: &Path, config: &SDeleteConfig) -> Vec<ContextMenuSpec> {
    let strings = strings_for(config.language);
    let icon = config
        .show_context_menu_icon
        .then(|| DEFAULT_WARNING_ICON.to_string());

    vec![
        build_delete_spec(
            r"*\shell",
            runner_exe,
            config,
            strings.secure_delete_menu,
            &icon,
        ),
        build_delete_spec(
            r"Directory\shell",
            runner_exe,
            config,
            strings.secure_delete_menu,
            &icon,
        ),
        build_drive_spec(runner_exe, config, &icon),
    ]
}

fn build_delete_spec(
    shell_root: &str,
    runner_exe: &Path,
    config: &SDeleteConfig,
    root_label: &str,
    icon: &Option<String>,
) -> ContextMenuSpec {
    if !config.use_pass_submenu {
        return ContextMenuSpec {
            shell_root: shell_root.to_string(),
            key_name: ROOT_SHELL_KEY_NAME.to_string(),
            mui_verb: root_label.to_string(),
            icon: icon.clone(),
            command: Some(build_shell_runner_command(
                runner_exe,
                SDeleteMode::Delete,
                config.passes,
            )),
            sub_items: Vec::new(),
        };
    }

    let strings = strings_for(config.language);
    let mut sub_items = Vec::new();

    sub_items.push(ContextMenuSpec {
        shell_root: String::new(),
        key_name: "00_default".to_string(),
        mui_verb: format_default_pass_label(strings, config.passes.get()),
        icon: icon.clone(),
        command: Some(build_shell_runner_command(
            runner_exe,
            SDeleteMode::Delete,
            config.passes,
        )),
        sub_items: Vec::new(),
    });

    let mut order = 1u8;
    for preset in SUBMENU_PASS_PRESETS {
        if preset == config.passes.get() {
            continue;
        }
        let pass_count = PassCount::new(preset).expect("Preset pass count must be valid.");
        sub_items.push(ContextMenuSpec {
            shell_root: String::new(),
            key_name: format!("{order:02}_passes_{preset}"),
            mui_verb: format_pass_label(strings, preset),
            icon: icon.clone(),
            command: Some(build_shell_runner_command(
                runner_exe,
                SDeleteMode::Delete,
                pass_count,
            )),
            sub_items: Vec::new(),
        });
        order += 1;
    }

    ContextMenuSpec {
        shell_root: shell_root.to_string(),
        key_name: ROOT_SHELL_KEY_NAME.to_string(),
        mui_verb: root_label.to_string(),
        icon: icon.clone(),
        command: None,
        sub_items,
    }
}

fn build_drive_spec(
    runner_exe: &Path,
    config: &SDeleteConfig,
    icon: &Option<String>,
) -> ContextMenuSpec {
    let strings = strings_for(config.language);
    let passes = config.passes.get();

    let clean_label = format!(
        "{}{passes}{}",
        strings.clean_free_space_prefix, strings.submenu_passes_suffix
    );
    let zero_label = format!(
        "{}{passes}{}",
        strings.zero_free_space_prefix, strings.submenu_passes_suffix
    );

    ContextMenuSpec {
        shell_root: r"Drive\shell".to_string(),
        key_name: ROOT_SHELL_KEY_NAME.to_string(),
        mui_verb: strings.erase_free_space_menu.to_string(),
        icon: icon.clone(),
        command: None,
        sub_items: vec![
            ContextMenuSpec {
                shell_root: String::new(),
                key_name: "01_clean_free_space".to_string(),
                mui_verb: clean_label,
                icon: icon.clone(),
                command: Some(build_shell_runner_command(
                    runner_exe,
                    SDeleteMode::CleanFreeSpace,
                    config.passes,
                )),
                sub_items: Vec::new(),
            },
            ContextMenuSpec {
                shell_root: String::new(),
                key_name: "02_zero_free_space".to_string(),
                mui_verb: zero_label,
                icon: icon.clone(),
                command: Some(build_shell_runner_command(
                    runner_exe,
                    SDeleteMode::ZeroFreeSpace,
                    config.passes,
                )),
                sub_items: Vec::new(),
            },
        ],
    }
}

fn open_classes_root(scope: RegistryScope) -> io::Result<RegKey> {
    let hive = match scope {
        RegistryScope::CurrentUser => RegKey::predef(HKEY_CURRENT_USER),
        RegistryScope::AllUsers => RegKey::predef(HKEY_LOCAL_MACHINE),
    };
    let (classes, _) = hive.create_subkey_with_flags(r"Software\Classes", KEY_WRITE)?;
    Ok(classes)
}

/// Applies all configured context menus to the selected Windows Registry scope.
pub fn apply_all(runner_exe: &Path, config: &SDeleteConfig) -> io::Result<()> {
    let _ = remove_all(config.registry_scope);
    let classes = open_classes_root(config.registry_scope)?;

    for spec in build_menu_specs(runner_exe, config) {
        write_spec(&classes, &spec)?;
    }

    Ok(())
}

/// Removes current and legacy SDelete-Gui context menus from the selected scope.
pub fn remove_all(scope: RegistryScope) -> io::Result<()> {
    let hive = match scope {
        RegistryScope::CurrentUser => RegKey::predef(HKEY_CURRENT_USER),
        RegistryScope::AllUsers => RegKey::predef(HKEY_LOCAL_MACHINE),
    };
    let classes = match hive.open_subkey_with_flags(r"Software\Classes", KEY_WRITE) {
        Ok(k) => k,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    for root in TARGET_ROOTS {
        let mut names = vec![ROOT_SHELL_KEY_NAME];
        names.extend_from_slice(&LEGACY_KEY_NAMES);
        for key_name in names {
            let full = format!(r"{root}\{key_name}");
            match classes.delete_subkey_all(&full) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => return Err(err),
            }
        }
    }

    Ok(())
}

fn write_spec(classes: &RegKey, spec: &ContextMenuSpec) -> io::Result<()> {
    let base_path = format!(r"{}\{}", spec.shell_root, spec.key_name);
    let (menu_key, _) = classes.create_subkey_with_flags(&base_path, KEY_WRITE)?;

    menu_key.set_value("MUIVerb", &spec.mui_verb)?;
    if let Some(icon) = &spec.icon {
        menu_key.set_value("Icon", icon)?;
    }

    if !spec.sub_items.is_empty() {
        // Empty SubCommands tells Windows Explorer to load inline child keys under shell.
        menu_key.set_value("SubCommands", &"")?;
        for child in &spec.sub_items {
            let child_path = format!(r"{base_path}\shell\{}", child.key_name);
            let (child_key, _) = classes.create_subkey_with_flags(&child_path, KEY_WRITE)?;
            child_key.set_value("MUIVerb", &child.mui_verb)?;
            if let Some(icon) = &child.icon {
                child_key.set_value("Icon", icon)?;
            }
            if let Some(cmd) = &child.command {
                let (cmd_key, _) = classes
                    .create_subkey_with_flags(format!(r"{child_path}\command"), KEY_WRITE)?;
                cmd_key.set_value("", cmd)?;
            }
        }
    } else if let Some(cmd) = &spec.command {
        let (cmd_key, _) =
            classes.create_subkey_with_flags(format!(r"{base_path}\command"), KEY_WRITE)?;
        cmd_key.set_value("", cmd)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_submenu_and_drive_specs_cleanly() {
        let cfg = SDeleteConfig::default();
        let specs = build_menu_specs(Path::new(r"C:\Tools\sdelete-gui.exe"), &cfg);
        assert_eq!(specs.len(), 3);

        // Files menu has icon, default (10), and 5 presets (1, 3, 5, 7, 35).
        assert_eq!(specs[0].icon.as_deref(), Some(DEFAULT_WARNING_ICON));
        assert_eq!(specs[0].sub_items.len(), 6);
        assert!(specs[0].sub_items.iter().any(|s| s
            .command
            .as_deref()
            .unwrap()
            .contains("--passes 35")));

        // Drive menu separates clean (-c) and zero (-z) free space.
        assert_eq!(specs[2].sub_items.len(), 2);
        assert!(specs[2].sub_items[0]
            .command
            .as_deref()
            .unwrap()
            .contains("--mode clean-free-space"));
        assert!(specs[2].sub_items[1]
            .command
            .as_deref()
            .unwrap()
            .contains("--mode zero-free-space"));
    }

    #[test]
    fn deduplicates_preset_matching_default_passes() {
        let cfg = SDeleteConfig {
            passes: PassCount::new(3).unwrap(),
            ..Default::default()
        };
        let specs = build_menu_specs(Path::new(r"C:\Tools\sdelete-gui.exe"), &cfg);
        assert_eq!(specs[0].sub_items.len(), 5);
    }

    #[test]
    fn supports_flat_menu_without_icon() {
        let cfg = SDeleteConfig {
            use_pass_submenu: false,
            show_context_menu_icon: false,
            ..Default::default()
        };
        let specs = build_menu_specs(Path::new(r"C:\Tools\sdelete-gui.exe"), &cfg);
        assert!(specs[0].sub_items.is_empty());
        assert!(specs[0].icon.is_none());
        assert!(specs[0].command.as_deref().unwrap().contains("--run"));
    }
}
