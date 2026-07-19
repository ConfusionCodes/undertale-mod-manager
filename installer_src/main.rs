use std::{
    env,
    path::{Path, PathBuf},
};

use eframe::{App, NativeOptions, icon_data};
use egui::{Button, Color32, Layout, ProgressBar, RichText, Vec2, ViewportBuilder, Widget};
use smol::{Executor, Task, channel::Receiver};

use crate::text::ALREADY_INSTALLED;

mod http;
mod text;

const WINDOW_SIZE: Vec2 = Vec2::new(300.0, 350.0);
const SUBFOLDER_NAME: &str = "UndertaleModManager";

#[derive(Debug)]
struct InstallerState {
    rx: Option<Receiver<f32>>,
    task: Option<Task<Result<(), http::Error>>>,
    progress: f32,

    create_shortcut: bool,
    install_path: String,
    block_install: bool,
    already_installed: bool,
}
impl InstallerState {
    fn new(_cc: &eframe::CreationContext) -> Box<Self> {
        let default_path = env::var_os("APPDATA")
            .or(env::var_os("LOCALAPPDATA"))
            .map(PathBuf::from)
            .or(std::env::home_dir());
        let mut initial_path =
            default_path.map_or(String::new(), |p| p.to_string_lossy().into_owned());
        if !initial_path.is_empty() {
            initial_path.push('\\');
            initial_path.push_str(SUBFOLDER_NAME);
        }
        Box::new(Self {
            rx: None,
            task: None,
            progress: 0.0,

            create_shortcut: true,
            install_path: initial_path,
            block_install: false,
            already_installed: false,
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
        if let Some(ref task) = self.task
            && let Some(ref rx) = self.rx
        {
            if task.is_finished()
                && let Some(task) = self.task.take()
            {
                let result = smol::block_on(task);
                ui.label(format!("{result:?}"));
            }
            if let Ok(progress) = rx.try_recv() {
                self.progress = progress;
            }
            ui.label("Installing... Please wait.");
            ProgressBar::new(self.progress).ui(ui);
            return;
        }

        ui.label(text::WELCOME);

        ui.checkbox(&mut self.create_shortcut, text::SHORTCUT);

        ui.label(text::INSTALL_PATH);
        ui.text_edit_singleline(&mut self.install_path);
        let (path, base_exists) = self.get_install_path();

        if !base_exists {
            ui.label(RichText::new(text::UNKNOWN_PATH).color(Color32::RED));
            self.block_install = true;
        } else {
            if let Ok(files) = path.read_dir() {
                let files: Vec<_> = files.filter_map(|f| f.ok()).collect();
                if files
                    .iter()
                    .any(|entry| entry.file_name() == "undertale_mod_manager.exe")
                {
                    ui.label(RichText::new(ALREADY_INSTALLED).color(Color32::YELLOW));
                    self.already_installed = true;
                }
            } else {
                eprintln!(
                    "Path '{}' was not found/not a directory, and was not caught earlier.",
                    path.display()
                );
            }
        }

        if self.already_installed {
            ui.label(RichText::new(ALREADY_INSTALLED).color(Color32::YELLOW));
        }
        ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
            let install_text = if self.already_installed {
                text::UPDATE
            } else {
                text::INSTALL
            };
            let install_button = ui.add_enabled(!self.block_install, Button::new(install_text));
            if install_button.clicked() {
                let (task, rx) = http::start_download(path);
                self.task = Some(task);
                self.rx = Some(rx);
                // let result = install(&path);
                // println!("{result:?}");
            }
            if ui.button("Cancel").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
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
