use std::{
    ffi::OsStr,
    fs::{self, read_dir},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use eframe::egui::{Color32, ImageSource, ahash::HashMap};
use egui::ahash::HashMapExt;
use fs_extra::dir::CopyOptions;
use image::{GenericImageView, ImageReader, ImageResult};
use lnk::ShellLink;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type AtomicProgress = Arc<Mutex<Progress<Error>>>;

#[derive(Debug, Clone, Copy)]
pub enum Progress<E: std::error::Error> {
    Loading(f32),
    Error(E),
    Done,
}
impl<E: std::error::Error> Default for Progress<E> {
    fn default() -> Self {
        Self::Loading(0.0)
    }
}

pub const ALLOWED_IMAGE_TYPES: [&str; 4] = ["ico", "png", "jpg", "jpeg"];

const UNDERTALE_STEAM_DIRECTORY: &str = "C:/Program Files (x86)/Steam/steamapps/common/Undertale";

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not create file/directory: {0}")]
    Copy(fs_extra::error::Error),
    #[error("Could not create file/directory: {0}")]
    Create(std::io::Error),
    #[error("Could not delete file/directory: {0}")]
    Delete(std::io::Error),
    #[error("Could not write to file: {0}")]
    Write(std::io::Error),
    #[error("Could not read file/directory: {0}")]
    Read(std::io::Error),
    #[error("Could not execute file: {0}")]
    Execute(std::io::Error),
    #[error("Could not convert to absolute path: {0}")]
    Canonicalize(std::io::Error),
    #[error("Could not read image data: {0}")]
    MainColor(image::ImageError),

    #[error("Could not find Undertale within the default Steam directory.")]
    SteamNotFound,
    #[error("Could not find 'data.win'.")]
    DataWinNotFound,
    #[error("Could not find 'UNDERTALE.exe'")]
    ExeNotFound,
    #[error("Could not list TEMP_DIRECTORY, despite mod {0} being found inside it.")]
    TempDirNotFound(String),
    #[error("Could not deserialize config file: {0}")]
    ConfigDeserialize(#[from] toml::de::Error),
    #[error("Could not serialize config file: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),
    #[error("Unrecognized file extension: {0}")]
    UnrecognizedExtension(String),
    #[error("File does not have an extension.")]
    NoExtension,
    #[error("File has no parent directory.")]
    NoParent,
    #[error("Could not find location of shortcut: {0}")]
    InvalidLink(lnk::Error),
    #[error("Could not strip the file prefix: {0}")]
    StripPrefix(std::path::StripPrefixError),
    #[error(
        "The mods directory does not exist, so the mod '{0}' cannot be inside it; aborting deletion for safety."
    )]
    NonexistentModPath(String),
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    pub instances: Vec<PathBuf>,
    pub icon_colors: HashMap<String, Color32>,
}
impl ConfigFile {
    pub fn new(files: FileManager) -> Self {
        Self {
            instances: vec![files.vanilla_dir.to_owned()],
            icon_colors: HashMap::new(),
        }
    }

    pub fn load(paths: &FileManager) -> Result<Self, Error> {
        let string = fs::read_to_string(paths.main_config).map_err(Error::Read)?;
        Ok(toml::from_str(&string)?)
    }

    pub fn save(&self, paths: &FileManager) -> Result<(), Error> {
        let string = toml::to_string_pretty(&self)?;
        fs::write(paths.main_config, string).map_err(Error::Write)?;
        Ok(())
    }

    pub fn get_color(&self, icon_path: &str) -> Option<Color32> {
        self.icon_colors.get(icon_path).copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileManager {
    pub icon_dir: &'static Path,
    pub mod_dir: &'static Path,
    pub vanilla_dir: &'static Path,
    pub temp_dir: &'static Path,
    pub main_config: &'static Path,
    pub instance_config: &'static Path,
}
impl FileManager {
    pub fn new() -> Self {
        Self {
            icon_dir: Self::get_or_create_dir("./icons/"),
            mod_dir: Self::get_or_create_dir("./mods/"),
            vanilla_dir: Self::get_or_create_dir("./vanilla/"),
            temp_dir: Self::get_or_create_dir("./temp/"),
            main_config: Path::new("./config.toml"),
            instance_config: Path::new("./instance.toml"),
        }
    }
    fn get_or_create_dir(string: &'static str) -> &'static Path {
        let path = Path::new(string);
        if !path.exists() {
            fs::create_dir_all(path).unwrap_or_else(|err| {
                panic!("Default directory '{path:?}' could not be created: {err}")
            });
        }
        path
    }
    pub fn instance_config_for(&self, mod_path: &Path) -> PathBuf {
        self.mod_dir.join(mod_path).join(self.instance_config)
    }
}
impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileManager {
    pub fn get_image(path: &str) -> ImageSource<'_> {
        format!("file://{path}").into()
    }

    pub fn show_icon_dir(&self) {
        if let Err(err) = std::process::Command::new("explorer")
            .arg(self.icon_dir)
            .spawn()
        {
            println!("Could not run command: {err}")
        }
    }

    pub fn copy_vanilla(&self, progress: AtomicProgress) -> Result<(), Error> {
        let undertale_dir = Self::locate_vanilla()?;
        self.copy_with_progress(&undertale_dir, self.vanilla_dir, progress)
    }
    pub fn copy_with_progress(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        progress: crate::AtomicProgress,
    ) -> Result<(), Error> {
        fs_extra::copy_items_with_progress(&[source_dir], target_dir, &CopyOptions::new(), |p| {
            if let Ok(mut progress) = progress.try_lock() {
                *progress = Progress::Loading((p.copied_bytes as f32) / (p.total_bytes as f32));
            }
            fs_extra::dir::TransitProcessResult::ContinueOrAbort
        })
        .map_err(Error::Copy)?;
        Ok(())
    }

    fn locate_vanilla() -> Result<PathBuf, Error> {
        let steam_dir = Path::new(UNDERTALE_STEAM_DIRECTORY);

        Self::is_undertale(steam_dir)?;
        Ok(steam_dir.to_path_buf())
    }

    pub fn copy_mod(&self, exe_path: &Path) -> Result<(), Error> {
        let source_dir = exe_path.parent().ok_or(Error::NoParent)?;
        let target_dir = self.mod_dir;
        let result = fs_extra::copy_items(&[source_dir], target_dir, &CopyOptions::new());
        match result {
            Ok(bytes) => println!("Mod '{}' copied ({} bytes).", exe_path.display(), bytes),
            Err(err) => println!("Failed to copy mod {}: {}", exe_path.display(), err),
        }

        Ok(())
    }
    pub fn delete_mod(&self, instance_path: &Path) -> Result<(), Error> {
        let full_instance_path = self.mod_dir.join(instance_path);
        if !full_instance_path.exists() {
            return Err(Error::NonexistentModPath(
                full_instance_path.to_string_lossy().into_owned(),
            ));
        }
        fs::remove_dir_all(full_instance_path).map_err(Error::Delete)
    }

    pub fn is_undertale(path: &Path) -> Result<(), Error> {
        let dir = match fs::read_dir(path) {
            Ok(dir) => dir,
            Err(err) => return Err(Error::Read(err)),
        };
        let files = dir
            .filter_map(|e| e.ok().map(|x| x.file_name()))
            .collect::<Vec<_>>();
        if !files.contains(&OsStr::new("data.win").to_owned()) {
            return Err(Error::DataWinNotFound);
        }
        if !files.contains(&OsStr::new("UNDERTALE.exe").to_owned()) {
            return Err(Error::ExeNotFound);
        }
        Ok(())
    }

    pub fn run_exe(path: &Path) -> Result<(), Error> {
        let absolute_path = path.canonicalize().map_err(Error::Canonicalize)?;
        let mut command = std::process::Command::new(&absolute_path);
        if let Some(parent) = absolute_path.parent() {
            command.current_dir(parent);
        }
        let exit_status = command
            .spawn()
            .map_err(Error::Execute)?
            .wait()
            .map_err(Error::Execute)?;
        println!("Process Exit Status: {exit_status}");
        Ok(())
    }

    pub fn get_exe_path(path: &Path) -> Result<PathBuf, Error> {
        let extension = path
            .extension()
            .map(|x| x.to_string_lossy().into_owned())
            .ok_or(Error::NoExtension)?;
        match extension.as_str() {
            "exe" => Ok(path.to_path_buf()),
            "lnk" => Self::exe_from_shortcut(path),
            "zip" => Self::exe_from_zip(path),
            x => Err(Error::UnrecognizedExtension(x.to_owned())),
        }
    }
    fn exe_from_shortcut(path: &Path) -> Result<PathBuf, Error> {
        let link =
            ShellLink::open(path, lnk::encoding::WINDOWS_1252).map_err(Error::InvalidLink)?;
        let location = link
            .link_target()
            .ok_or(Error::InvalidLink(lnk::Error::UnexpectedEof(
                "Link did not have a location",
            )))?;

        Ok(PathBuf::from(location))
    }
    fn exe_from_zip(path: &Path) -> Result<PathBuf, Error> {
        todo!()
    }

    pub fn add_icon(&self, icon_path: &Path, config: &mut ConfigFile) -> Result<(), Error> {
        let name = icon_path
            .file_name()
            .expect("Path already confirmed to have an extension");
        fs::copy(icon_path, self.icon_dir.join(name)).map_err(Error::Write)?;
        Ok(())
    }
    pub fn get_icons(&self) -> Vec<String> {
        let Ok(dir) = read_dir(self.icon_dir) else {
            println!("Failed to read icon directory");
            return Vec::new();
        };
        dir.filter_map(|x| {
            x.ok()
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ALLOWED_IMAGE_TYPES.iter().any(|x| OsStr::new(x) == ext))
                })
                .map(|e| e.path().to_string_lossy().into_owned())
        })
        .collect()
    }
    fn get_unique(path: &Path) -> PathBuf {
        let mut count = 0;
        let mut path = path.to_path_buf();
        let file_name = path
            .file_name()
            .unwrap_or(OsStr::new("Unnamed"))
            .to_os_string();
        loop {
            if path.exists() {
                let mut new_file_name = file_name.clone();
                new_file_name.push(OsStr::new(&format!(" ({})", count)));
                let new_path = path.with_file_name(new_file_name);
                path = new_path;
                count += 1;
            } else {
                return path;
            }
        }
    }
}
