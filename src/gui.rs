use std::path::PathBuf;

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Visuals};

use crate::config::{
    Language, PassCount, RegistryScope, SDeleteConfig, ThemeMode, MAX_PASSES, MIN_PASSES,
};
use crate::i18n::strings_for;
use crate::registry;
use crate::sdelete_bin::{
    ensure_sdelete_binary, find_existing_sdelete, is_process_elevated, remove_managed_binary,
};

/// Launches the eframe/egui configuration window.
pub fn run_gui() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SDelete Gui")
            .with_inner_size([460.0, 500.0])
            .with_min_inner_size([420.0, 460.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SDelete Gui",
        options,
        Box::new(|cc| {
            let app = SDeleteGuiApp::new();
            apply_theme(&cc.egui_ctx, app.config.theme);
            Ok(Box::new(app))
        }),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusKind {
    Neutral,
    Success,
    Error,
}

struct SDeleteGuiApp {
    config: SDeleteConfig,
    log_path_input: String,
    sdelete_path: Option<PathBuf>,
    is_admin: bool,
    status_message: String,
    status_kind: StatusKind,
}

impl SDeleteGuiApp {
    fn new() -> Self {
        let config = SDeleteConfig::load();
        let strings = strings_for(config.language);
        let log_path_input = config.log_file_path.to_string_lossy().to_string();
        let sdelete_path = find_existing_sdelete();
        let is_admin = is_process_elevated();

        Self {
            config,
            log_path_input,
            sdelete_path,
            is_admin,
            status_message: strings.ready_status.to_string(),
            status_kind: StatusKind::Neutral,
        }
    }

    fn handle_enable(&mut self) {
        let strings = strings_for(self.config.language);
        self.config.log_file_path = PathBuf::from(self.log_path_input.trim());
        self.config.normalize();
        self.log_path_input = self.config.log_file_path.to_string_lossy().to_string();

        if self.config.registry_scope == RegistryScope::AllUsers && !self.is_admin {
            self.status_message = strings.permission_error.to_string();
            self.status_kind = StatusKind::Error;
            return;
        }

        match ensure_sdelete_binary(self.config.registry_scope) {
            Ok(path) => self.sdelete_path = Some(path),
            Err(err) => {
                self.status_message = err.to_string();
                self.status_kind = StatusKind::Error;
                return;
            }
        }

        if let Err(err) = self.config.save() {
            self.status_message = err.to_string();
            self.status_kind = StatusKind::Error;
            return;
        }

        let Ok(runner_exe) = std::env::current_exe() else {
            self.status_message = "Failed to resolve executable path.".to_string();
            self.status_kind = StatusKind::Error;
            return;
        };

        match registry::apply_all(&runner_exe, &self.config) {
            Ok(()) => {
                self.status_message = format!(
                    "{}{}{}",
                    strings.configured_status_prefix,
                    self.config.passes.get(),
                    strings.configured_status_suffix
                );
                self.status_kind = StatusKind::Success;
            }
            Err(err) => {
                self.status_message = format!("{} ({err})", strings.permission_error);
                self.status_kind = StatusKind::Error;
            }
        }
    }

    fn handle_disable(&mut self) {
        let strings = strings_for(self.config.language);
        if self.config.registry_scope == RegistryScope::AllUsers && !self.is_admin {
            self.status_message = strings.permission_error.to_string();
            self.status_kind = StatusKind::Error;
            return;
        }

        match registry::remove_all(self.config.registry_scope) {
            Ok(()) => {
                let _ = remove_managed_binary();
                self.sdelete_path = find_existing_sdelete();
                self.status_message = strings.unconfigured_status.to_string();
                self.status_kind = StatusKind::Success;
            }
            Err(err) => {
                self.status_message = format!("{} ({err})", strings.permission_error);
                self.status_kind = StatusKind::Error;
            }
        }
    }
}

impl eframe::App for SDeleteGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let strings = strings_for(self.config.language);

        egui::TopBottomPanel::top("header_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(strings.app_title).strong().size(17.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_label = match self.config.theme {
                        ThemeMode::Dark => strings.theme_light,
                        ThemeMode::Light => strings.theme_dark,
                    };
                    if ui.button(theme_label).clicked() {
                        self.config.theme = match self.config.theme {
                            ThemeMode::Dark => ThemeMode::Light,
                            ThemeMode::Light => ThemeMode::Dark,
                        };
                        apply_theme(ctx, self.config.theme);
                        let _ = self.config.save();
                    }

                    egui::ComboBox::from_id_salt("language_selector")
                        .width(84.0)
                        .selected_text(match self.config.language {
                            Language::English => "English",
                            Language::Spanish => "Espanol",
                            Language::German => "Deutsch",
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(
                                    &mut self.config.language,
                                    Language::English,
                                    "English",
                                )
                                .changed()
                            {
                                let _ = self.config.save();
                            }
                            if ui
                                .selectable_value(
                                    &mut self.config.language,
                                    Language::Spanish,
                                    "Espanol",
                                )
                                .changed()
                            {
                                let _ = self.config.save();
                            }
                            if ui
                                .selectable_value(
                                    &mut self.config.language,
                                    Language::German,
                                    "Deutsch",
                                )
                                .changed()
                            {
                                let _ = self.config.save();
                            }
                        });
                });
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            ui.horizontal(|ui| {
                ui.label(strings.passes_label);
                let mut raw_passes = self.config.passes.get();
                if ui
                    .add(egui::DragValue::new(&mut raw_passes).range(MIN_PASSES..=MAX_PASSES))
                    .changed()
                {
                    self.config.passes = PassCount::clamped(u32::from(raw_passes));
                }
            });

            ui.separator();

            ui.checkbox(
                &mut self.config.confirm_before_delete,
                strings.confirm_before_delete,
            );
            ui.checkbox(
                &mut self.config.sequential_execution,
                strings.sequential_execution,
            );
            ui.checkbox(&mut self.config.use_pass_submenu, strings.use_pass_submenu);
            ui.checkbox(
                &mut self.config.show_context_menu_icon,
                strings.show_context_menu_icon,
            );
            ui.checkbox(&mut self.config.enable_audit_log, strings.enable_audit_log);

            ui.add_enabled_ui(self.config.enable_audit_log, |ui| {
                ui.label(strings.log_file_path);
                ui.add(
                    egui::TextEdit::singleline(&mut self.log_path_input)
                        .desired_width(f32::INFINITY),
                );
            });

            ui.separator();

            ui.label(strings.scope_label);
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut self.config.registry_scope,
                    RegistryScope::CurrentUser,
                    strings.scope_current_user,
                );
                ui.radio_value(
                    &mut self.config.registry_scope,
                    RegistryScope::AllUsers,
                    strings.scope_all_users,
                );
            });
            if self.config.registry_scope == RegistryScope::AllUsers && !self.is_admin {
                ui.label(
                    RichText::new(strings.admin_required_note)
                        .color(Color32::from_rgb(217, 119, 6)),
                );
            }

            ui.separator();

            match &self.sdelete_path {
                Some(path) => {
                    ui.label(format!("{} {}", strings.sdelete_found, path.display()));
                }
                None => {
                    ui.label(strings.sdelete_missing);
                }
            }

            ui.add_space(4.0);

            ui.columns(2, |cols| {
                if cols[0]
                    .add_sized(
                        [cols[0].available_width(), 32.0],
                        egui::Button::new(strings.enable_button),
                    )
                    .clicked()
                {
                    self.handle_enable();
                }
                if cols[1]
                    .add_sized(
                        [cols[1].available_width(), 32.0],
                        egui::Button::new(strings.disable_button),
                    )
                    .clicked()
                {
                    self.handle_disable();
                }
            });

            ui.add_space(6.0);

            let status_color = match (self.status_kind, self.config.theme) {
                (StatusKind::Success, ThemeMode::Dark) => Color32::from_rgb(74, 222, 128),
                (StatusKind::Success, ThemeMode::Light) => Color32::from_rgb(21, 128, 61),
                (StatusKind::Error, ThemeMode::Dark) => Color32::from_rgb(248, 113, 113),
                (StatusKind::Error, ThemeMode::Light) => Color32::from_rgb(185, 28, 28),
                (StatusKind::Neutral, ThemeMode::Dark) => Color32::from_rgb(203, 213, 225),
                (StatusKind::Neutral, ThemeMode::Light) => Color32::from_rgb(51, 65, 85),
            };
            ui.label(RichText::new(&self.status_message).color(status_color));

            ui.add_space(4.0);
            ui.label(RichText::new(strings.ssd_caveat_note).weak().small());
        });
    }
}

/// Applies clean neutral dark or light desktop visuals without decorative rounding.
fn apply_theme(ctx: &egui::Context, theme: ThemeMode) {
    let mut visuals = match theme {
        ThemeMode::Dark => {
            let mut v = Visuals::dark();
            v.panel_fill = Color32::from_rgb(24, 26, 31);
            v.window_fill = Color32::from_rgb(24, 26, 31);
            v.widgets.inactive.bg_fill = Color32::from_rgb(38, 42, 50);
            v.widgets.hovered.bg_fill = Color32::from_rgb(50, 56, 67);
            v.widgets.active.bg_fill = Color32::from_rgb(37, 99, 235);
            v
        }
        ThemeMode::Light => {
            let mut v = Visuals::light();
            v.panel_fill = Color32::from_rgb(255, 255, 255);
            v.window_fill = Color32::from_rgb(255, 255, 255);
            v.widgets.inactive.bg_fill = Color32::from_rgb(236, 239, 244);
            v.widgets.hovered.bg_fill = Color32::from_rgb(220, 226, 234);
            v.widgets.active.bg_fill = Color32::from_rgb(37, 99, 235);
            v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(203, 213, 225));
            v
        }
    };

    let radius = CornerRadius::same(3);
    visuals.widgets.noninteractive.corner_radius = radius;
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.active.corner_radius = radius;
    visuals.widgets.open.corner_radius = radius;

    ctx.set_visuals(visuals);
}
