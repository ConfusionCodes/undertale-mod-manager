use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};

use eframe::{NativeOptions, icon_data};
use egui::{Button, Color32, Layout, RichText, Vec2, ViewportBuilder};
use reqwest::{Response, StatusCode};

const WINDOW_SIZE: Vec2 = Vec2::new(300.0, 350.0);
const RELEASE_URL: &str =
    r"https://api.github.com/repos/ConfusionCodes/undertale-mod-manager/releases/latest";

fn main() -> eframe::Result {
    let default_path = env::var_os("APPDATA")
        .or(env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .or(std::env::home_dir());
    let initial_path = default_path.map_or(String::new(), |p| p.to_string_lossy().into_owned());
    let icon = icon_data::from_png_bytes(include_bytes!("../assets/logo.png"));
    eframe::run_ui_native(
        "Undertale Mod Manager Installer",
        NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size(WINDOW_SIZE)
                .with_icon(icon.unwrap_or_default())
                .with_maximize_button(false),
            // .with_resizable(false),
            ..NativeOptions::default()
        },
        move |ui, _| {
            if ui
                .memory(|mem| mem.data.get_temp("isntall".into()))
                .unwrap_or(false)
            {
                return;
            }

            ui.label("Welcome to the Undertale Mod Manager Installer. Just configure the settings below and press \"install\", and the latest version will be downloaded and installed.");

            let mut shortcut: bool = ui
                .memory_mut(|mem| mem.data.get_temp("shortcut".into()))
                .unwrap_or(true);
            ui.checkbox(&mut shortcut, "Create Desktop Shortcut");
            ui.memory_mut(|mem| mem.data.insert_temp("shortcut".into(), shortcut));

            ui.label("Insallation path: (This is where all the files will go. It should already be filled with a reasonable default.)");
            let mut path: String = ui
                .memory_mut(|mem| mem.data.get_temp("path".into()))
                .unwrap_or(initial_path.clone());
            ui.text_edit_singleline(&mut path);
            let base_path = Path::new(&path);
            let base_exists = base_path.exists() && base_path.is_dir();
            let full_path = base_path.join("UndertaleModManager");
            ui.memory_mut(|mem| mem.data.insert_temp("path".into(), path));

            let mut block_install = false;
            if !base_exists {
                ui.label(RichText::new("Could not find the specified path. If you typed this manually, check for spelling errors.").color(Color32::RED));
                block_install = true;
            }
            if full_path.exists() && full_path.is_dir() {
                if let Ok(files) = full_path.read_dir() {
                    let files: Vec<_> = files.filter_map(|f| f.ok()).collect();
                    if files
                        .iter()
                        .any(|entry| entry.file_name() == "undertale_mod_manager.exe")
                    {
                        ui.label(RichText::new("\
                        You seem to already have Undertale Mod Manager installed in this directory. \
                        Installing here will overwrite the currently installed version. \
                        Your mods and configuration settings will not be altered.\
                        ").color(Color32::YELLOW));
                    }
                } else {
                    ui.label(RichText::new("Could not access the specified folder. If you want to install here, try running the installer as administrator.").color(Color32::RED));
                    block_install = true;
                }
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                let install_button = ui.add_enabled(!block_install, Button::new("Install"));
                if install_button.clicked() {
                    ui.memory_mut(|mem| mem.data.insert_temp("install".into(), true));
                    // install_new_version();
                }
                if ui.button("Cancel").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        },
    )
}

struct ReleaseInfo {}

async fn install(path: &Path) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(RELEASE_URL).send().await?;
    if response.status() != StatusCode::OK && response.status() != StatusCode::FOUND {
        eprintln!("Could not find resource.")
    }
    let data = response.text().await?;
    println!("data: {data}");

    Ok(())
}
