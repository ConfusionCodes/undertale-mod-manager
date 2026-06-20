use std::ffi::OsStr;

use eframe::egui::{
    Align2, Area, Button, CentralPanel, Color32, Frame, Grid, Image, Key, Layout, Margin, Order,
    Panel, ProgressBar, ScrollArea, Sense, Shadow, TextEdit, TextStyle, Ui, Vec2, Widget,
};

use crate::{
    ModManager, Progress,
    files::{self, ConfigFile, FileManager},
    modifications::{Program, ProgramWidget},
    style,
};

#[derive(Debug, Default, Clone, Copy)]
pub enum Screen {
    #[default]
    Loading,
    Home,
}
impl Screen {
    pub fn display(self, state: &mut ModManager, ui: &mut Ui, frame: &mut eframe::Frame) {
        match self {
            Screen::Loading => loading(state, ui, frame),
            Screen::Home => home(state, ui, frame),
        }
    }
}

pub fn loading(state: &mut ModManager, ui: &mut Ui, _frame: &mut eframe::Frame) {
    CentralPanel::default_margins()
        .frame(Frame::new().fill(Color32::BLACK))
        .show_inside(ui, |ui| {
            Area::new("loading_content".into())
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Loading...");

                        match state.progress.lock().as_deref() {
                            Ok(Progress::Loading(p)) => {
                                ProgressBar::new(*p).animate(true).ui(ui);
                            }
                            Ok(Progress::Error(error)) => {
                                ProgressBar::new(0.0).animate(false).ui(ui);
                                ui.label(format!("Error copying files: {}", error));
                            }
                            Ok(Progress::Done) => {
                                state.screen = Screen::Home;
                                state.vanilla_loaded = true;
                            }
                            Err(err) => println!("Could not acquire lock; Arc is poisoned: {err}"),
                        }
                    });
                });
        });
}

pub fn home(state: &mut ModManager, ui: &mut Ui, _frame: &mut eframe::Frame) {
    Panel::top("toolbar")
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(Color32::BLACK)
                .outer_margin(Margin {
                    left: 0,
                    right: 0,
                    top: 0,
                    bottom: 8,
                })
                .inner_margin(Margin::same(8)),
        )
        .show_inside(ui, |ui| {
            if ui.button("Add Mod").clicked() {
                state.add_menu_open = true;
            } else if state.add_menu_open {
                add_mod_menu(state, ui);
            }
        });
    Panel::right("mod_options")
        .resizable(false)
        .show_separator_line(false)
        .min_size(style::INFO_PANEL_WIDTH)
        .frame(
            Frame::new()
                .fill(Color32::BLACK)
                .outer_margin(Margin {
                    left: 8,
                    right: 0,
                    top: 0,
                    bottom: 0,
                })
                .inner_margin(Margin::same(16)),
        )
        .show_inside(ui, |ui| {
            if let Some(selected_index) = state.selected_index {
                ui.vertical_centered(|ui| {
                    let selected_mod = &mut state.mods[selected_index];
                    let image_response = Image::new(selected_mod.image())
                        .sense(Sense::click())
                        .ui(ui)
                        .on_hover_cursor(eframe::egui::CursorIcon::PointingHand);
                    if image_response.clicked() {
                        state.icon_menu_open = true;
                    } else if state.icon_menu_open {
                        icon_menu(
                            selected_mod,
                            &mut state.icon_menu_open,
                            &state.files,
                            &mut state.config,
                            ui,
                        );
                    }
                    if state.renaming {
                        let response = TextEdit::multiline(&mut selected_mod.name)
                            .desired_rows(1)
                            .font(TextStyle::Heading)
                            .ui(ui);
                        if response.has_focus() {
                            if ui.input_mut(|i| i.key_pressed(Key::Enter)) {
                                ui.memory_mut(|mem| mem.surrender_focus(response.id));
                            }
                        } else if response.lost_focus() {
                            state.renaming = false;
                            if let Err(err) = Program::save(selected_mod, &state.files) {
                                println!("Faied to save program: {}", err);
                            };
                            selected_mod.name.retain(|c| c != '\n' && c != '\r');
                        } else {
                            response.request_focus();
                        }
                    } else {
                        if ui.heading(&selected_mod.name).double_clicked() {
                            state.renaming = true;
                        };
                    }
                    if ui.button("Play").clicked() {
                        let result = FileManager::run_exe(&selected_mod.exe_path(state.files));
                        if let Err(err) = result {
                            println!("Failed to run program: {err}")
                        }
                    };
                    if ui.button("Rename").clicked() {
                        state.renaming = true;
                    };
                    let delete_button = ui
                        .add_enabled(!selected_mod.is_vanilla(), Button::new("Delete"))
                        .on_disabled_hover_text("You can't delete vanilla Undertale.");
                    if delete_button.clicked() {
                        state.delete_confirmation_open = true;
                    } else if state.delete_confirmation_open {
                        let delete = delete_confirm_menu(
                            &selected_mod.name,
                            &mut state.delete_confirmation_open,
                            ui,
                        );
                        if delete {
                            state.queue_deletion(selected_index);
                        }
                    }
                });
            } else {
                state.icon_menu_open = false;
            }
        });
    CentralPanel::default_margins()
        .frame(
            Frame::new()
                .fill(Color32::BLACK)
                .inner_margin(Margin::same(8)),
        )
        .show_inside(ui, |ui| {
            let rect = ui.max_rect();
            let bg_response = ui.interact(rect, "mod_panel_bg".into(), Sense::click());
            Grid::new("mod_panel").show(ui, |ui| {
                for (i, program) in state.mods.iter().enumerate() {
                    let is_selected = state.selected_index.is_some_and(|x| x == i);
                    ui.vertical(|ui| {
                        if ProgramWidget::new(program, is_selected).ui(ui).clicked() {
                            state.selected_index = Some(i);
                        };
                    });
                }
            });
            if bg_response.clicked() {
                state.selected_index = None;
            }
            //
        });
}

fn add_mod_menu(state: &mut ModManager, ui: &mut Ui) {
    let area_size = style::popup_size(ui);
    popup(ui, &mut state.add_menu_open, |ui| {
        let (rect, _response) = ui.allocate_exact_size(area_size, Sense::CLICK);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Drop Files Here...",
            TextStyle::Heading.resolve(ui.style()),
            Color32::WHITE,
        );
    });
    ui.input(|i| {
        for file in &i.raw.dropped_files {
            let Some(path) = &file.path else {
                println!("Could not resolve dropped file path.");
                continue;
            };
            let exe_path = FileManager::get_exe_path(path);
            match exe_path {
                Ok(exe_path) => match Program::from_exe_path(&exe_path, &state.files) {
                    Ok(program) => {
                        let _ = state.files.copy_mod(&exe_path);
                        state.config.instances.push(program.path.clone());
                        state.mods.push(program);
                        state.add_menu_open = false;
                    }
                    Err(err) => println!("Program Failed: {err}"),
                },
                Err(err) => println!("path failed: {err}"),
            }
        }
    });
}

fn icon_menu(
    selected_mod: &mut Program,
    open_flag: &mut bool,
    files: &FileManager,
    config: &mut ConfigFile,
    ui: &mut Ui,
) {
    let mut open = true;
    let area_size = style::popup_size(ui);
    popup(ui, open_flag, |ui| {
        ui.set_min_size(area_size);
        ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.label("Open Folder");
                ui.label("Lorem");
                ui.label("Ipsum");
            });
            ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    Grid::new("icons").show(ui, |ui| {
                        let default_image = Image::new(style::DEFAULT_IMAGE)
                            .fit_to_exact_size(style::TILE_SIZE)
                            .maintain_aspect_ratio(true);

                        if Button::new(default_image).ui(ui).clicked() {
                            selected_mod.image_path = None;
                            open = false;
                        }
                        for icon in files.get_icons() {
                            let icon_image = Image::new(FileManager::get_image(&icon))
                                .fit_to_exact_size(style::TILE_SIZE)
                                .maintain_aspect_ratio(true);
                            if Button::new(icon_image).ui(ui).clicked() {
                                selected_mod.image_path = Some(icon);
                                open = false;
                            };
                        }
                    });
                });

                //
            });
        });
    });
    *open_flag = open;
    ui.input(|i| {
        for file in &i.raw.dropped_files {
            let Some(path) = &file.path else {
                println!("Could not resolve dropped file path.");
                continue;
            };
            if path.extension().is_some_and(|ext| {
                files::ALLOWED_IMAGE_TYPES
                    .iter()
                    .any(|x| OsStr::new(x) == ext)
            }) {
                let copy_result = files.add_icon(path, config);
                if let Err(err) = copy_result {
                    println!("Failed to add icon: {err}")
                };
            }
        }
    });
}

fn delete_confirm_menu(name: &str, open_flag: &mut bool, ui: &mut Ui) -> bool {
    let mut delete = false;
    let mut open = true;
    popup(ui, open_flag, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading(format!("Are you sure you want to delete ‘{name}’?"));
            ui.label("(This action cannot be undone!)");
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    open = false;
                };
                if ui.button("Confirm").clicked() {
                    delete = true;
                    open = false;
                };
            })
        });
    });
    *open_flag = open;
    delete
}

fn popup(ui: &mut Ui, open_flag: &mut bool, add_contents: impl FnOnce(&mut Ui)) {
    let response = Area::new("icon_popup".into())
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .default_size(style::popup_size(ui))
        .order(Order::Foreground)
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style())
                .fill(Color32::BLACK)
                .stroke((8.0, Color32::RED))
                .corner_radius(0.0)
                .shadow(Shadow {
                    offset: [0, 0],
                    blur: 24,
                    spread: 8,
                    color: Color32::from_black_alpha(128),
                })
                .show(ui, add_contents);
        })
        .response;
    if response.clicked_elsewhere() {
        *open_flag = false;
    }
}
