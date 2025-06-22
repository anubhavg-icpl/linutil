// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod theme;
mod config;
mod utils;

use std::collections::HashMap;
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use linutil_core::{get_tabs, Command as LinutilCommand};
use config::AppConfig;
use utils::{execute_command_safe, execute_script_safe};

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
    pub command_content: String, // Store the actual command content here
    pub task_list: String,
    pub multi_select: bool,
    pub has_children: bool,
    pub id: String,
}

static TABS_CACHE: Mutex<Option<Vec<TabInfo>>> = Mutex::new(None);
static APP_CONFIG: Mutex<AppConfig> = Mutex::new(AppConfig {
    skip_confirmation: false,
    override_validation: true,
    size_bypass: true,
    mouse: true,
    bypass_root: true,
});

fn load_tabs_with_validation(_validate: bool) -> Result<Vec<TabInfo>, String> {
    let mut cache = TABS_CACHE.lock().unwrap();
    
    if let Some(ref cached_tabs) = *cache {
        return Ok(cached_tabs.clone());
    }
    
    // For desktop app, always override validation to prevent compatibility loops
    // The validate parameter is ignored to ensure stability
    let tabs = get_tabs(true); // Always use override validation (true = override, false = validate)
    let mut result = Vec::new();
    
    for tab in tabs.iter() {
        let mut tab_info = TabInfo {
            name: tab.name.clone(),
            entries: Vec::new(),
        };
        
        // Get all entries from the tree
        for node in tab.tree.root().descendants() {
            let node_value = node.value();
            if node_value.name != "root" {
                let (command_type, command_content) = match &node_value.command {
                    LinutilCommand::Raw(cmd) => ("raw".to_string(), cmd.clone()),
                    LinutilCommand::LocalFile { executable, args, file } => {
                        let script_content = std::fs::read_to_string(file)
                            .unwrap_or_else(|_| format!("Script file: {}", file.display()));
                        ("script".to_string(), format!("{}|{}|{}", executable, args.join(" "), script_content))
                    },
                    LinutilCommand::None => ("directory".to_string(), String::new()),
                };
                
                let entry = EntryInfo {
                    name: node_value.name.clone(),
                    description: node_value.description.clone(),
                    command_type,
                    command_content,
                    task_list: node_value.task_list.clone(),
                    multi_select: node_value.multi_select,
                    has_children: node.has_children(),
                    id: format!("{:?}", node.id()),
                };
                tab_info.entries.push(entry);
            }
        }
        
        result.push(tab_info);
    }
    
    *cache = Some(result.clone());
    Ok(result)
}

#[tauri::command]
fn get_all_tabs(_override_validation: Option<bool>) -> Result<Vec<TabInfo>, String> {
    // For desktop app, always override validation to prevent loops
    // The override_validation parameter is accepted for compatibility but ignored
    load_tabs_with_validation(true)
}

#[tauri::command]
fn execute_command(tab_name: String, entry_name: String) -> Result<String, String> {
    // Use cached tabs to avoid infinite loops
    let tabs = load_tabs_with_validation(true)?;
    
    // Find the command in the cached tabs
    for tab in tabs.iter() {
        if tab.name == tab_name {
            // Search for the entry
            for entry in &tab.entries {
                if entry.name == entry_name {
                    match entry.command_type.as_str() {
                        "raw" => {
                            return execute_raw_command(&entry.command_content);
                        }
                        "script" => {
                            let parts: Vec<&str> = entry.command_content.split('|').collect();
                            if parts.len() >= 2 {
                                let executable = parts[0];
                                let args: Vec<String> = if parts[1].is_empty() {
                                    Vec::new()
                                } else {
                                    parts[1].split_whitespace().map(|s| s.to_string()).collect()
                                };
                                return execute_script_command(executable, &args);
                            } else {
                                return Err("Invalid script command format".to_string());
                            }
                        }
                        "directory" => {
                            return Err("Cannot execute directory".to_string());
                        }
                        _ => {
                            return Err("Unknown command type".to_string());
                        }
                    }
                }
            }
        }
    }
    
    Err("Command not found".to_string())
}

fn execute_raw_command(cmd: &str) -> Result<String, String> {
    execute_command_safe(cmd)
}

fn execute_script_command(executable: &str, args: &[String]) -> Result<String, String> {
    execute_script_safe(executable, args)
}

#[tauri::command]
fn get_system_info() -> Result<HashMap<String, String>, String> {
    let mut info = HashMap::new();
    
    // Get OS information
    if let Ok(output) = Command::new("uname").arg("-a").output() {
        info.insert("system".to_string(), String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    
    // Get distribution information
    if let Ok(output) = Command::new("lsb_release").arg("-d").output() {
        info.insert("distribution".to_string(), String::from_utf8_lossy(&output.stdout).trim().to_string());
    } else if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let name = line.strip_prefix("PRETTY_NAME=").unwrap_or("")
                    .trim_matches('"');
                info.insert("distribution".to_string(), name.to_string());
                break;
            }
        }
    }
    
    Ok(info)
}

#[tauri::command]
fn get_command_preview(tab_name: String, entry_name: String) -> Result<String, String> {
    // Use cached tabs to avoid infinite loops
    let tabs = load_tabs_with_validation(true)?;
    
    // Find the entry in the cached tabs
    for tab in tabs.iter() {
        if tab.name == tab_name {
            for entry in &tab.entries {
                if entry.name == entry_name {
                    match entry.command_type.as_str() {
                        "raw" => {
                            return Ok(format!("Raw Command:\n{}\n\nDescription:\n{}", entry.command_content, entry.description));
                        }
                        "script" => {
                            let parts: Vec<&str> = entry.command_content.split('|').collect();
                            if parts.len() >= 3 {
                                let content = parts[2];
                                return Ok(format!("Script Preview:\n{}\n\nDescription:\n{}", content, entry.description));
                            } else {
                                return Ok(format!("Script Command: {} {}\n\nDescription:\n{}", 
                                    parts.get(0).unwrap_or(&""), 
                                    parts.get(1).unwrap_or(&""), 
                                    entry.description));
                            }
                        }
                        "directory" => {
                            return Ok(format!("Directory: {}\n\nDescription:\n{}", entry.name, entry.description));
                        }
                        _ => {
                            return Err("Unknown command type".to_string());
                        }
                    }
                }
            }
        }
    }
    
    Err("Command not found".to_string())
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
    let mut cache = TABS_CACHE.lock().unwrap();
    *cache = None;
    
    Ok(())
}

#[tauri::command]
fn clear_cache() -> Result<(), String> {
    let mut cache = TABS_CACHE.lock().unwrap();
    *cache = None;
    Ok(())
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
            clear_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}