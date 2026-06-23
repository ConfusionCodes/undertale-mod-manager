use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use eframe::{
    App, CreationContext, NativeOptions,
    egui::{Color32, FontData, FontDefinitions, FontFamily},
};

use crate::{
    files::{AtomicProgress, ConfigFile, FileManager, Progress},
    modifications::Program,
    screens::Screen,
};

mod files;
mod modifications;
mod screens;
mod style;

#[derive(Debug, Default)]
pub struct ModManager {
    pub files: FileManager,
    pub screen: Screen,
    pub vanilla_loaded: bool,
    pub mods: Vec<Program>,
    pub selected_index: Option<usize>,
    pub renaming: bool,
    pub add_menu_open: bool,
    pub icon_menu_open: bool,
    pub delete_confirmation_open: bool,
    pub progress: AtomicProgress,
    pub config: ConfigFile,
    deletion_index: Option<usize>,
}
impl ModManager {
    fn new(cc: &CreationContext) -> Box<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        set_font!(cc, "../assets/8bitoperator_jve.ttf");
        style::set_font_settings(cc, 18.0, FontFamily::Monospace);

        let files = FileManager::new();

        let mut screen = Screen::Loading;
        let progress = Arc::new(Mutex::new(Progress::default()));
        let mut vanilla_loaded = false;
        if FileManager::is_undertale(&files.vanilla_dir.join("Undertale")).is_ok() {
            vanilla_loaded = true;
            screen = Screen::Home;
        } else {
            Self::copy_vanilla_files(progress.clone(), files);
        }

        let mut config = ConfigFile::load(&files).unwrap_or_else(|err| {
            println!("Could not load config file. Creating new one: {err}");
            let config = ConfigFile::default();
            let contents = toml::to_string(&config).unwrap();
            let _ = dbg!(fs::write(files.main_config, contents));
            config
        });
        let mut mods = Program::load_all(&config, &files)
            .unwrap_or_else(|err| panic!("Could not load programs: {err}"));

        if mods.is_empty() {
            let vanilla = Program::vanilla();
            config.instances.push(vanilla.path.clone());
            mods.push(vanilla);
        }
        let state = Self {
            files,
            screen,
            vanilla_loaded,
            selected_index: None,
            mods,
            renaming: false,
            add_menu_open: false,
            icon_menu_open: false,
            delete_confirmation_open: false,
            progress: progress.clone(),
            config,
            deletion_index: None,
        };
        Box::new(state)
    }

    fn copy_vanilla_files(inner_progress: AtomicProgress, files: FileManager) {
        thread::spawn(move || {
            let result = files.copy_vanilla(inner_progress.clone());
            if let Ok(mut progress) = inner_progress.lock() {
                if let Err(err) = result {
                    *progress = Progress::Error(err);
                } else {
                    *progress = Progress::Done;
                }
            }
        });
    }

    pub fn selected_mod(&self) -> Option<&Program> {
        self.selected_index.map(|i| {
            self.mods
                .get(i)
                .expect("Selected index should not exceed installed mods.")
        })
    }

    pub fn queue_deletion(&mut self, index: usize) {
        if index >= self.mods.len() {
            println!("Could not find mod to delete with index '{index}'.");
            return;
        }
        self.deletion_index = Some(index);
    }

    pub fn delete_queued(&mut self) {
        if let Some(index) = self.deletion_index {
            let program = self.mods.remove(index);
            if let Err(err) = self.files.delete_mod(&program.path) {
                println!(
                    "And error occurred trying to delete mod '{}': {}",
                    program.name, err
                );
            } else {
                self.config.instances.remove(index);
            };
            self.deletion_index = None;
            self.selected_index = None;
        }
    }
}

impl App for ModManager {
    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        self.selected_mod()
            .and_then(|m| m.image_path.as_ref())
            .and_then(|p| self.config.get_color(p))
            .unwrap_or(Color32::RED)
            .to_normalized_gamma_f32()
    }
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        self.delete_queued();
        self.screen.display(self, ui, frame);
    }
    fn on_exit(&mut self) {
        if let Err(err) = self.config.save(&self.files) {
            println!("Failed to save config file: {err}\nDump: {:?}", self.config);
        }
        if let Err(err) = Program::save_all(&self.mods, &self.files) {
            println!(
                "Failed to save instance metadata: {err}\nDump: {:?}",
                self.mods
            );
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "Undertale Mod Manager",
        NativeOptions::default(),
        Box::new(|cc| Ok(ModManager::new(cc))),
    )
}
