use std::{
    fs::{self},
    path::{Path, PathBuf},
};

use eframe::egui::{
    Align, Color32, Image, ImageSource, Rect, Sense, TextStyle, Vec2, Widget, pos2,
    text::LayoutJob, vec2,
};
use serde::{Deserialize, Serialize};

use crate::{
    files::{self, ConfigFile, FileManager},
    style::{self},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramVariant {
    Standalone,
    // installed: bool
    Mod(bool),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub name: String,
    pub path: PathBuf,
    pub variant: ProgramVariant,
    pub image_path: Option<String>,
    pub exe_name: PathBuf,
}
impl Program {
    pub fn from_exe_path(exe_path: &Path, paths: &FileManager) -> Result<Self, files::Error> {
        let exe_name = exe_path.file_name().expect("File terminated in '..'");
        let grandparent = exe_path
            .parent()
            .and_then(|x| x.parent())
            .ok_or(files::Error::NoParent)?;
        let path_relative = exe_path
            .strip_prefix(grandparent)
            .map_err(files::Error::StripPrefix)?;

        let name = exe_path
            .file_stem()
            .unwrap_or(exe_name)
            .to_string_lossy()
            .into_owned();
        println!("{name}");
        let program = Self {
            name,
            exe_name: PathBuf::from(exe_name),
            image_path: None,
            path: path_relative
                .parent()
                .map(Path::to_path_buf)
                .ok_or(files::Error::NoParent)?,
            variant: ProgramVariant::Standalone,
        };
        Self::save(&program, paths)?;
        Ok(program)
    }
    pub fn vanilla() -> Self {
        Self {
            name: String::from("Undertale"),
            path: PathBuf::from("../vanilla/Undertale"),
            variant: ProgramVariant::Standalone,
            image_path: None,
            exe_name: PathBuf::from("UNDERTALE.exe"),
        }
    }

    pub fn load_all(config: &ConfigFile, paths: &FileManager) -> Result<Vec<Self>, files::Error> {
        config
            .instances
            .iter()
            .map(|path| Self::load(path, &paths))
            .collect()
    }
    pub fn save_all(programs: &[Self], paths: &FileManager) -> Result<(), files::Error> {
        for program in programs {
            program.save(paths)?;
        }
        Ok(())
    }
    fn load(path: &Path, paths: &FileManager) -> Result<Program, files::Error> {
        let path = paths.instance_config_for(path);

        let string = fs::read_to_string(path).map_err(files::Error::Read)?;
        Ok(toml::from_str(&string)?)
    }
    pub fn save(&self, paths: &FileManager) -> Result<(), files::Error> {
        let path = paths.instance_config_for(&self.path);
        let string = toml::to_string(self)?;
        fs::write(path, string).map_err(files::Error::Write)
    }

    pub fn exe_path(&self, paths: FileManager) -> PathBuf {
        paths.mod_dir.join(&self.path).join(&self.exe_name)
    }

    pub fn image(&self) -> ImageSource<'_> {
        self.image_path
            .as_deref()
            .map(FileManager::get_image)
            .unwrap_or(style::DEFAULT_IMAGE)
    }

    pub fn is_vanilla(&self) -> bool {
        self.path.starts_with("../vanilla")
    }
}

pub struct ProgramWidget<'a> {
    program: &'a Program,
    selected: bool,
}
impl<'a> ProgramWidget<'a> {
    pub fn new(program: &'a Program, selected: bool) -> Self {
        Self { program, selected }
    }
}
impl<'a> Widget for ProgramWidget<'a> {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let desired_size = style::TILE_SIZE;
        let line_height = TextStyle::Body.resolve(ui.style()).size;
        // let text_rect = ui.painter().text(
        //     rect.center_bottom(),
        //     Align2::CENTER_BOTTOM,
        //     &self.program.name,
        //     TextStyle::Heading.resolve(ui.style()),
        //     Color32::WHITE,
        // );
        let image_space = vec2(desired_size.x, desired_size.y - line_height);
        let image = Image::new(self.program.image())
            .maintain_aspect_ratio(true)
            .shrink_to_fit();
        let effective_image_space = image_space - Vec2::splat(style::MOD_TILE_IMAGE_PADDING * 2.);
        let image_size = image
            .load_and_calc_size(ui, effective_image_space)
            .unwrap_or(effective_image_space);

        let mut job = LayoutJob::simple(
            self.program.name.clone(),
            TextStyle::Heading.resolve(ui.style()),
            Color32::WHITE,
            desired_size.x,
        );
        job.halign = Align::Center;

        let text = ui.painter().layout_job(job);

        let desired_size = vec2(
            desired_size.x,
            (desired_size.y - line_height) + text.size().y,
        );

        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());
        if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(
                rect,
                0,
                if self.selected {
                    Color32::GRAY
                } else {
                    Color32::BLACK
                },
            );
            let image_rect = Rect::from_min_size(
                pos2(
                    rect.center().x - image_size.x / 2.,
                    rect.top() + style::MOD_TILE_IMAGE_PADDING,
                ),
                image_size,
            );
            image.paint_at(ui, image_rect);
            ui.painter().galley(
                pos2(
                    image_rect.center().x,
                    image_rect.bottom() + style::MOD_TILE_IMAGE_PADDING,
                ),
                text,
                Color32::WHITE,
            );
        }
        response
    }
}
