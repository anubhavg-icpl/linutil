use crate::cli::Args;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub skip_confirmation: bool,
    pub override_validation: bool,
    pub size_bypass: bool,
    pub mouse: bool,
    pub bypass_root: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            skip_confirmation: false,
            override_validation: true, // Default to true for desktop to prevent compatibility loops
            size_bypass: true,
            mouse: true,
            bypass_root: true,
        }
    }
}

impl From<Args> for AppConfig {
    fn from(args: Args) -> Self {
        Self {
            skip_confirmation: args.skip_confirmation,
            override_validation: args.override_validation,
            size_bypass: args.size_bypass,
            mouse: args.mouse,
            bypass_root: args.bypass_root,
        }
    }
}

impl AppConfig {
    #[allow(dead_code)]
    pub fn load_from_file(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        
        Ok(config)
    }

    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        
        Ok(())
    }
}