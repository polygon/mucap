use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use miniserde::{Deserialize, Serialize, json};
use nih_plug::{debug::nih_log, nih_warn};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub scale_factor: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self { scale_factor: 1.0 }
    }
}

pub struct ConfigStore {
    config_file: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Self {
        let cfg_dir = ProjectDirs::from("matelab", "matelab", "mucap")
            .expect("Failed to get project directories");

        let config_dir = cfg_dir.config_dir();
        let _ = std::fs::create_dir_all(config_dir);

        Self {
            config_file: config_dir.join("config.json"),
        }
    }

    pub fn get_config(&self) -> Config {
        std::fs::read_to_string(&self.config_file)
            .ok()
            .and_then(|content| json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn set_config(&mut self, config: &Config) {
        let json_str = json::to_string(config);
        if let Err(e) = std::fs::write(&self.config_file, json_str) {
            nih_warn!(
                "Failed to save config: {:?}, Error: {}",
                &self.config_file,
                e
            );
        } else {
            nih_log!("Saved config: {:?}", &self.config_file);
        }
    }
}
