use std::collections::BTreeMap;

use egui::{FontId, TextStyle};

macro_rules! text {
    ($($name:ident = $text:literal);+$(;)? ) => {
        $(pub const $name: &str = $text;)+
    };
}

text![
    WELCOME = "Welcome to the Undertale Mod Manager Installer. Just configure the settings below and press \"install\", and the latest version will be downloaded and installed.";
    INSTALL_PATH = "Install location. (This is where all the files will go.)";
    CHANGE_INSTALL_PATH = "Change install location.";
    UNKNOWN_PATH = "Could not find the specified path. If you typed this manually, check for spelling errors.";
    ALREADY_INSTALLED = "You seem to already have Undertale Mod Manager installed in this directory. Installing here will update the currently installed version. Your mods and configuration settings will not be altered.";
    // NO_PERMISSION = "Could not access the specified folder. If you want to install here, try running the installer as administrator.";
    // NOT_A_DIRECTORY = "The specified path is not a directory/folder.";
    //Buttons
    SHORTCUT = "Create Desktop Shortcut";
    INSTALL = "Install";
    UPDATE = "Update";
    CANCEL = "Cancel";
];

pub fn text_styles() -> BTreeMap<TextStyle, FontId> {
    use egui::FontFamily::{Monospace, Proportional};

    [
        (TextStyle::Small, FontId::new(12.0, Proportional)),
        (TextStyle::Body, FontId::new(18.0, Proportional)),
        (TextStyle::Button, FontId::new(18.0, Proportional)),
        (TextStyle::Heading, FontId::new(26.0, Proportional)),
        (TextStyle::Monospace, FontId::new(18.0, Monospace)),
    ]
    .into()
}
