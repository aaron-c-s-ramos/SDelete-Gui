use crate::config::Language;

/// Holds all localized UI, context-menu, and dialog strings.
#[derive(Debug, Clone, Copy)]
pub struct Strings {
    pub app_title: &'static str,
    pub passes_label: &'static str,
    pub confirm_before_delete: &'static str,
    pub sequential_execution: &'static str,
    pub use_pass_submenu: &'static str,
    pub show_context_menu_icon: &'static str,
    pub enable_audit_log: &'static str,
    pub log_file_path: &'static str,
    pub scope_label: &'static str,
    pub scope_current_user: &'static str,
    pub scope_all_users: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,
    pub enable_button: &'static str,
    pub disable_button: &'static str,
    pub ready_status: &'static str,
    pub configured_status_prefix: &'static str,
    pub configured_status_suffix: &'static str,
    pub unconfigured_status: &'static str,
    pub permission_error: &'static str,
    pub sdelete_found: &'static str,
    pub sdelete_missing: &'static str,
    pub secure_delete_menu: &'static str,
    pub erase_free_space_menu: &'static str,
    pub submenu_default_prefix: &'static str,
    pub submenu_passes_suffix: &'static str,
    pub submenu_one_pass: &'static str,
    pub clean_free_space_prefix: &'static str,
    pub zero_free_space_prefix: &'static str,
    pub clean_free_space_short: &'static str,
    pub zero_free_space_short: &'static str,
    pub confirm_title: &'static str,
    pub confirm_irreversible: &'static str,
    pub confirm_more_prefix: &'static str,
    pub confirm_more_suffix: &'static str,
    pub admin_required_note: &'static str,
    pub ssd_caveat_note: &'static str,
}

const ENGLISH: Strings = Strings {
    app_title: "SDelete Gui",
    passes_label: "Default overwrite passes",
    confirm_before_delete: "Ask for confirmation before deletion",
    sequential_execution: "Erase multiple selected files sequentially",
    use_pass_submenu: "Add pass-count submenu (1, 3, 5, 7, 35)",
    show_context_menu_icon: "Show warning icon in context menu",
    enable_audit_log: "Log deleted items with timestamp",
    log_file_path: "Audit log file path",
    scope_label: "Context menu scope",
    scope_current_user: "Current user (no elevation)",
    scope_all_users: "All users (administrator)",
    theme_dark: "Dark",
    theme_light: "Light",
    enable_button: "Enable",
    disable_button: "Disable",
    ready_status: "Ready.",
    configured_status_prefix: "Enabled. Default overwrite count: ",
    configured_status_suffix: " pass(es).",
    unconfigured_status: "Disabled. Context menus removed.",
    permission_error: "Access denied. Run as Administrator for All Users scope.",
    sdelete_found: "Sysinternals SDelete binary:",
    sdelete_missing: "Sysinternals SDelete not found locally. Enable will download sdelete64.exe.",
    secure_delete_menu: "Secure Delete",
    erase_free_space_menu: "Erase Free Space (SDelete)",
    submenu_default_prefix: "Default (",
    submenu_passes_suffix: " passes)",
    submenu_one_pass: "1 pass",
    clean_free_space_prefix: "Clean free space (-c, ",
    zero_free_space_prefix: "Zero free space (-z, ",
    clean_free_space_short: "clean free space",
    zero_free_space_short: "zero free space",
    confirm_title: "Confirm Secure Delete",
    confirm_irreversible: "This action cannot be undone.",
    confirm_more_prefix: "...and ",
    confirm_more_suffix: " more item(s).",
    admin_required_note: "All Users scope requires Administrator privileges.",
    ssd_caveat_note: "Note: SSD wear-leveling limits software overwrites. Live-file slack space is not cleared by SDelete.",
};

const SPANISH: Strings = Strings {
    app_title: "SDelete Gui",
    passes_label: "Pasadas de sobrescritura",
    confirm_before_delete: "Pedir confirmacion antes de borrar",
    sequential_execution: "Borrar multiples archivos de forma secuencial",
    use_pass_submenu: "Agregar submenu de pasadas (1, 3, 5, 7, 35)",
    show_context_menu_icon: "Mostrar icono de advertencia en el menu",
    enable_audit_log: "Registrar elementos borrados con fecha y hora",
    log_file_path: "Ruta del archivo de registro",
    scope_label: "Alcance del menu contextual",
    scope_current_user: "Usuario actual (sin elevacion)",
    scope_all_users: "Todos los usuarios (administrador)",
    theme_dark: "Oscuro",
    theme_light: "Claro",
    enable_button: "Habilitar",
    disable_button: "Deshabilitar",
    ready_status: "Listo.",
    configured_status_prefix: "Habilitado. Pasadas predeterminadas: ",
    configured_status_suffix: " pasada(s).",
    unconfigured_status: "Deshabilitado. Menus contextuales eliminados.",
    permission_error: "Acceso denegado. Ejecute como Administrador para Todos los usuarios.",
    sdelete_found: "Binario de Sysinternals SDelete:",
    sdelete_missing: "SDelete no encontrado localmente. Habilitar descargara sdelete64.exe.",
    secure_delete_menu: "Borrado Seguro",
    erase_free_space_menu: "Borrar espacio libre (SDelete)",
    submenu_default_prefix: "Predeterminado (",
    submenu_passes_suffix: " pasadas)",
    submenu_one_pass: "1 pasada",
    clean_free_space_prefix: "Limpiar espacio libre (-c, ",
    zero_free_space_prefix: "Llenar con ceros (-z, ",
    clean_free_space_short: "limpieza de espacio libre",
    zero_free_space_short: "llenado con ceros",
    confirm_title: "Confirmar Borrado Seguro",
    confirm_irreversible: "Esta accion no se puede deshacer.",
    confirm_more_prefix: "...y ",
    confirm_more_suffix: " elemento(s) mas.",
    admin_required_note: "Todos los usuarios requiere permisos de Administrador.",
    ssd_caveat_note: "Nota: el desgaste en SSD limita las sobrescrituras de software.",
};

const GERMAN: Strings = Strings {
    app_title: "SDelete Gui",
    passes_label: "Standard-Durchlaeufe",
    confirm_before_delete: "Vor dem Loeschen bestaetigen",
    sequential_execution: "Mehrere Dateien nacheinander loeschen",
    use_pass_submenu: "Untermenue fuer Durchlaeufe (1, 3, 5, 7, 35)",
    show_context_menu_icon: "Warnsymbol im Kontextmenue anzeigen",
    enable_audit_log: "Geloeschte Elemente mit Zeitstempel protokollieren",
    log_file_path: "Pfad der Protokolldatei",
    scope_label: "Bereich des Kontextmenues",
    scope_current_user: "Aktueller Benutzer (ohne Admin)",
    scope_all_users: "Alle Benutzer (Administrator)",
    theme_dark: "Dunkel",
    theme_light: "Hell",
    enable_button: "Aktivieren",
    disable_button: "Deaktivieren",
    ready_status: "Bereit.",
    configured_status_prefix: "Aktiviert. Standard-Durchlaeufe: ",
    configured_status_suffix: " Durchlauf/Durchlaeufe.",
    unconfigured_status: "Deaktiviert. Kontextmenues entfernt.",
    permission_error: "Zugriff verweigert. Als Administrator fuer Alle Benutzer ausfuehren.",
    sdelete_found: "Sysinternals SDelete-Datei:",
    sdelete_missing: "SDelete lokal nicht gefunden. Aktivieren laedt sdelete64.exe herunter.",
    secure_delete_menu: "Sicher Loeschen",
    erase_free_space_menu: "Freien Speicher loeschen (SDelete)",
    submenu_default_prefix: "Standard (",
    submenu_passes_suffix: " Durchlaeufe)",
    submenu_one_pass: "1 Durchlauf",
    clean_free_space_prefix: "Freien Speicher bereinigen (-c, ",
    zero_free_space_prefix: "Freien Speicher nullen (-z, ",
    clean_free_space_short: "Bereinigung des freien Speichers",
    zero_free_space_short: "Nullen des freien Speichers",
    confirm_title: "Sicheres Loeschen bestaetigen",
    confirm_irreversible: "Diese Aktion kann nicht rueckgaengig gemacht werden.",
    confirm_more_prefix: "...und ",
    confirm_more_suffix: " weitere Elemente.",
    admin_required_note: "Alle Benutzer erfordert Administratorrechte.",
    ssd_caveat_note: "Hinweis: SSD Wear-Leveling begrenzt Software-Ueberschreibungen.",
};

/// Returns the static string table for the selected language.
pub const fn strings_for(lang: Language) -> &'static Strings {
    match lang {
        Language::English => &ENGLISH,
        Language::Spanish => &SPANISH,
        Language::German => &GERMAN,
    }
}

/// Formats the submenu label for a specific pass count.
pub fn format_pass_label(strings: &Strings, passes: u8) -> String {
    if passes == 1 {
        strings.submenu_one_pass.to_string()
    } else {
        format!(
            "{passes}{}",
            strings.submenu_passes_suffix.trim_end_matches(')')
        )
    }
}

/// Formats the submenu label for the configured default pass count.
pub fn format_default_pass_label(strings: &Strings, passes: u8) -> String {
    format!(
        "{}{passes}{}",
        strings.submenu_default_prefix, strings.submenu_passes_suffix
    )
}
