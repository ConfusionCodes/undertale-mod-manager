#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};

use eframe::{App, NativeOptions, icon_data};
use egui::{
    Button, CentralPanel, Color32, Layout, ProgressBar, RichText, TextEdit, Vec2, ViewportBuilder,
    Widget,
};
use smol::{Task, channel::Receiver};

mod http;
mod text;

const WINDOW_SIZE: Vec2 = Vec2::new(600.0, 400.0);
const SUBFOLDER_NAME: &str = "UndertaleModManager";
pub const TEMP_EXE_NAME: &str = "undertale_mod_manager.exe.downloading";
const EXE_NAME: &str = "undertale_mod_manager.exe";
const SHORTCUT_NAME: &str = "Undertale Mod Manager.lnk";

#[derive(Debug)]
struct InstallerState {
    rx: Option<Receiver<f32>>,
    task: Option<Task<Result<(), http::Error>>>,
    progress: f32,

    initial_install_path: String,
    desktop_path: Option<PathBuf>,

    create_shortcut: bool,
    change_install_path: bool,
    install_path: String,
    already_installed: bool,
    install_error: String,
}
impl InstallerState {
    fn new(cc: &eframe::CreationContext) -> Box<Self> {
        let default_path = dirs::data_dir();
        let mut initial_path =
            default_path.map_or(String::new(), |p| p.to_string_lossy().into_owned());
        if !initial_path.is_empty() {
            initial_path.push('\\');
            initial_path.push_str(SUBFOLDER_NAME);
        }

        cc.egui_ctx
            .all_styles_mut(move |style| style.text_styles = text::text_styles());

        Box::new(Self {
            rx: None,
            task: None,
            progress: 0.0,

            initial_install_path: initial_path.clone(),
            desktop_path: dirs::desktop_dir(),

            create_shortcut: true,
            change_install_path: initial_path.is_empty(),
            install_path: initial_path,
            already_installed: false,
            install_error: String::new(),
        })
    }
    fn get_install_path(&self) -> (PathBuf, bool) {
        let path = Path::new(&self.install_path).to_path_buf();
        let is_valid = path.parent().is_some_and(|p| p.exists() && p.is_dir());
        (path, is_valid)
    }
}
impl App for InstallerState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        CentralPanel::default_margins().show(ui, |ui| {
            if handle_install(self, ui) {
                return;
            } else if !self.install_error.is_empty() {
                ui.label(
                    RichText::new(&self.install_error)
                        .heading()
                        .color(Color32::RED),
                );
                return;
            }

            ui.label(text::WELCOME);
            ui.add_space(16.0);
            if self.desktop_path.is_some() {
                ui.checkbox(&mut self.create_shortcut, text::SHORTCUT);
            }
            ui.add_space(16.0);
            if !self.initial_install_path.is_empty() {
                ui.checkbox(&mut self.change_install_path, text::CHANGE_INSTALL_PATH);
            } else {
                ui.label(text::INSTALL_PATH);
            }
            ui.add_enabled(
                self.change_install_path,
                TextEdit::singleline(&mut self.install_path).desired_width(ui.available_width()),
            );
            let (path, base_exists) = self.get_install_path();

            let mut block_install = false;
            if !base_exists {
                ui.label(RichText::new(text::UNKNOWN_PATH).color(Color32::RED));
                block_install = true;
            } else {
                if let Ok(files) = path.read_dir() {
                    let files: Vec<_> = files.filter_map(|f| f.ok()).collect();
                    if let Some(file) = files
                        .iter()
                        .find(|e| e.file_name() == TEMP_EXE_NAME)
                        .map(|f| f.path())
                    {
                        let _ = std::fs::remove_file(file);
                    }
                    if files.iter().any(|entry| entry.file_name() == EXE_NAME) {
                        self.already_installed = true;
                    }
                } else {
                    self.already_installed = false;
                }
            }

            if self.already_installed {
                ui.label(RichText::new(text::ALREADY_INSTALLED).color(Color32::YELLOW));
            }

            ui.with_layout(Layout::left_to_right(egui::Align::Max), |ui| {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Button,
                    egui::FontId::new(36.0, egui::FontFamily::Proportional),
                );

                if ui
                    .button(RichText::new(text::CANCEL).text_style(egui::TextStyle::Button))
                    .clicked()
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }

                ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                    let install_text = if self.already_installed {
                        text::UPDATE
                    } else {
                        text::INSTALL
                    };
                    let install_button = ui.add_enabled(
                        !block_install,
                        Button::new(
                            RichText::new(install_text).text_style(egui::TextStyle::Button),
                        ),
                    );
                    if install_button.clicked() {
                        if !path.exists()
                            && let Err(err) = std::fs::create_dir(&path)
                        {
                            eprintln!("Could not create containing directory: {err}");
                        };
                        let (task, rx) = http::start_download(path);
                        self.task = Some(task);
                        self.rx = Some(rx);
                    }
                });
            });
        });
    }
}

fn handle_install(state: &mut InstallerState, ui: &mut egui::Ui) -> bool {
    if let Some(ref task) = state.task
        && let Some(ref rx) = state.rx
    {
        if task.is_finished()
            && let Some(task) = state.task.take()
        {
            let result = smol::block_on(task);
            if let Err(err) = result {
                state.install_error = format!("Failed to install the mod manager: {err}");
                return false;
            }
            let exe_path = Path::new(&state.install_path).join(EXE_NAME);
            let result = std::fs::rename(
                Path::new(&state.install_path).join(TEMP_EXE_NAME),
                exe_path.clone(),
            );
            if let Err(err) = result {
                state.install_error = format!("Failed to install the mod manager: {err}");
                return false;
            }

            if let Some(ref desktop_path) = state.desktop_path {
                let link = mslnk::ShellLink::new(exe_path).unwrap();
                link.create_lnk(desktop_path.join(SHORTCUT_NAME)).unwrap();
            }
            return false;
        }
        match rx.try_recv() {
            Ok(progress) => state.progress = progress,
            Err(err) => eprintln!("Could not fetch progress: {}", err),
        }
        if let Ok(progress) = rx.try_recv() {
            state.progress = progress;
            println!("Progress: {progress}");
        }
        ui.label("Installing... Please wait.");
        ProgressBar::new(state.progress)
            .animate(true)
            .show_percentage()
            .ui(ui);
        return true;
    }
    false
}

fn main() -> eframe::Result {
    let icon = icon_data::from_png_bytes(include_bytes!("../assets/logo.png"));
    eframe::run_native(
        "Undertale Mod Manager Installer",
        NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size(WINDOW_SIZE)
                .with_icon(icon.unwrap_or_default())
                .with_maximize_button(false),
            // .with_resizable(false),
            ..NativeOptions::default()
        },
        Box::new(|cc| Ok(InstallerState::new(cc))),
    )
}
