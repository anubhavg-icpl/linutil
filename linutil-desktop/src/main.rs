// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod theme;
mod config;
mod utils;
mod core_integration;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use config::AppConfig;
use core_integration::{
    load_tabs_with_core, 
    execute_command_with_core, 
    get_command_preview_with_core, 
    clear_tabs_cache,
    get_system_info as get_system_info_core,
    CommandExecutionResult
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TabInfo {
    pub name: String,
    pub entries: Vec<EntryInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntryInfo {
    pub name: String,
    pub description: String,
    pub command_type: String,
    pub command_content: String,
    pub task_list: String,
    pub multi_select: bool,
    pub has_children: bool,
    pub id: String,
}

static APP_CONFIG: Mutex<AppConfig> = Mutex::new(AppConfig {
    skip_confirmation: false,
    override_validation: true, // Skip validation by default for desktop
    size_bypass: true,
    mouse: true,
    bypass_root: true,
});

#[tauri::command]
fn get_all_tabs() -> Result<Vec<TabInfo>, String> {
    let config = APP_CONFIG.lock().unwrap();
    load_tabs_with_core(&config)
}

#[tauri::command]
fn execute_command(tab_name: String, entry_name: String) -> Result<CommandExecutionResult, String> {
    let config = APP_CONFIG.lock().unwrap();
    execute_command_with_core(&tab_name, &entry_name, &config)
}

#[tauri::command]
fn get_command_preview(tab_name: String, entry_name: String) -> Result<String, String> {
    get_command_preview_with_core(&tab_name, &entry_name)
}

#[tauri::command]
fn get_system_info() -> Result<std::collections::HashMap<String, String>, String> {
    Ok(get_system_info_core())
}

#[tauri::command]
fn get_app_config() -> Result<AppConfig, String> {
    let config = APP_CONFIG.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
fn update_app_config(new_config: AppConfig) -> Result<(), String> {
    let mut config = APP_CONFIG.lock().unwrap();
    *config = new_config;
    
    // Clear cache to force reload with new validation settings
    clear_tabs_cache();
    
    Ok(())
}

#[tauri::command]
fn clear_cache() -> Result<(), String> {
    clear_tabs_cache();
    Ok(())
}

#[tauri::command]
fn refresh_tabs() -> Result<Vec<TabInfo>, String> {
    clear_tabs_cache();
    let config = APP_CONFIG.lock().unwrap();
    load_tabs_with_core(&config)
}

#[tauri::command]
fn validate_environment() -> Result<std::collections::HashMap<String, bool>, String> {
    let mut validation = std::collections::HashMap::new();
    
    // Check if running as root
    validation.insert("is_root".to_string(), 
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout).trim() == "0"
            })
            .unwrap_or(false)
    );
    
    // Check if common tools are available
    let tools = ["curl", "git", "sh", "bash"];
    for tool in &tools {
        validation.insert(format!("has_{}", tool), 
            std::process::Command::new("which")
                .arg(tool)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        );
    }
    
    // Check if package managers are available
    let package_managers = ["apt-get", "dnf", "pacman", "zypper", "apk"];
    for pm in &package_managers {
        validation.insert(format!("has_{}", pm), 
            std::process::Command::new("which")
                .arg(pm)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        );
    }
    
    Ok(validation)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_all_tabs,
            execute_command,
            get_system_info,
            get_command_preview,
            get_app_config,
            update_app_config,
            clear_cache,
            refresh_tabs,
            validate_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}