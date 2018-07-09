use toml;
use xdg;

use std::fs::File;
use std::path::PathBuf;
use std::io::Read;
use std::str::FromStr;

use clap::ArgMatches;

#[derive(Deserialize)]
pub struct Dimension {
    pub height: u32,
    pub width: u32,
    pub x: u32,
    pub y: u32,
}

#[derive(Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Element {
    Command {
        command: String,
        width: Option<u32>,
    },
    Repeat {
        command: String,
        time: u32,
        width: Option<u32>,
    },
    Fixed {
        size: u32,
    },
    Seperator {
        sep: String,
    },
    Right
}

impl Element {
    pub fn get_width(&self) -> Option<u32> {
        match self {
            Element::Command { width, .. } |
                Element::Repeat { width, .. } => {
                    return *width;
                }
            _ => {
                return None;
            }
        }
    }
}

#[derive(Deserialize)]
pub struct Config {
    pub dimensions: Dimension,
    pub font: String,
    pub elements: Vec<Element>
}

fn from_path(path: PathBuf) -> Config {
    let mut file = File::open(&path).expect(&format!("Could not open file {}", path.display()));
    let mut content = String::new();
    file.read_to_string(&mut content).expect("Could not read config file");
    return toml::from_str(&content).expect("Config file is an invalid toml file");
}

pub fn get_config(matches: ArgMatches) -> Config {
    let mut config = matches.value_of("config").and_then(|path| {
        return Some(from_path(path.into()));
    }).unwrap_or_else(|| {
        let dirs = xdg::BaseDirectories::with_prefix("rustdock").expect("Could not find config paths");

        if !dirs.get_config_home().exists() {
            return Config {
                font: String::from(" "), // An empty font crashes dzen2
                dimensions: Dimension {
                    height: 20,
                    width: 400,
                    x: 0,
                    y: 0,
                },
                elements: vec![]
            }
        }

        let mut config_path = dirs.get_config_home();
        config_path.push("./config.toml");

        return from_path(config_path);
    });
    if let Some(width) = matches.value_of("width").and_then(|x| u32::from_str(x).ok()) {
        config.dimensions.width = width;
    }
    if let Some(height) = matches.value_of("height").and_then(|x| u32::from_str(x).ok()) {
        config.dimensions.height = height;
    }
    if let Some(x) = matches.value_of("xpos").and_then(|x| u32::from_str(x).ok()) {
        config.dimensions.x = x;
    }
    if let Some(y) = matches.value_of("ypos").and_then(|x| u32::from_str(x).ok()) {
        config.dimensions.y = y;
    }

    return config;
}
