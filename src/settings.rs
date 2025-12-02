use std::fs;

use dirs::config_dir;
use log::{error, trace};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub reasoning: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: "sk-TESTKEY".to_string(),
            model: "google/gemma-3-27b-it".to_string(),
            temperature: 0.5,
            max_tokens: 1000,
            reasoning: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = config_dir()
            .map(|mut path| {
                path.push("moon");
                path.push("settings.json");
                path
            })
            .unwrap();
        trace!("Trying to load from {:?}", path);

        match path.exists() {
            true => match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(settings) => {
                        trace!("Loaded settings");
                        settings
                    }
                    Err(e) => {
                        error!("Error parsing config: {}", e);
                        Self::default()
                    }
                },
                Err(e) => {
                    error!("Error reading config: {}", e);
                    Self::default()
                }
            },
            false => {
                let default = Self::default();
                error!("Config not found. Writing default");
                default.save().unwrap_or_else(|e| error!("{e}"));
                default
            }
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = config_dir().ok_or("Unable to find config directory")?;

        let fullmoon_dir = config_dir.join("moon");
        if !fullmoon_dir.exists() {
            fs::create_dir_all(&fullmoon_dir)?;
        }

        let config_path = fullmoon_dir.join("settings.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;

        Ok(())
    }
}
