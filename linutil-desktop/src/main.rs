// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use linutil_core::{get_tabs, Command as LinutilCommand};

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
    pub task_list: String,
    pub multi_select: bool,
    pub has_children: bool,
    pub id: String,
}

static TABS_CACHE: Mutex<Option<Vec<TabInfo>>> = Mutex::new(None);

fn load_tabs() -> Result<Vec<TabInfo>, String> {
    let mut cache = TABS_CACHE.lock().unwrap();
    
    if let Some(ref cached_tabs) = *cache {
        return Ok(cached_tabs.clone());
    }
    
    let tabs = get_tabs(true);
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
                let entry = EntryInfo {
                    name: node_value.name.clone(),
                    description: node_value.description.clone(),
                    command_type: match &node_value.command {
                        LinutilCommand::Raw(_) => "raw".to_string(),
                        LinutilCommand::LocalFile { .. } => "script".to_string(),
                        LinutilCommand::None => "directory".to_string(),
                    },
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
fn get_all_tabs() -> Result<Vec<TabInfo>, String> {
    load_tabs()
}

#[tauri::command]
fn execute_command(tab_name: String, entry_name: String) -> Result<String, String> {
    // Load tabs fresh each time to avoid thread issues
    let tabs = get_tabs(true);
    
    // Find the command in the tabs
    for tab in tabs.iter() {
        if tab.name == tab_name {
            // Search for the entry in the tree
            for node in tab.tree.root().descendants() {
                let node_value = node.value();
                if node_value.name == entry_name {
                    match &node_value.command {
                        LinutilCommand::Raw(cmd) => {
                            return execute_raw_command(cmd);
                        }
                        LinutilCommand::LocalFile { executable, args, .. } => {
                            return execute_script_command(executable, args);
                        }
                        LinutilCommand::None => {
                            return Err("Cannot execute directory".to_string());
                        }
                    }
                }
            }
        }
    }
    
    Err("Command not found".to_string())
}

fn execute_raw_command(cmd: &str) -> Result<String, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        Ok(format!("Success:\n{}", stdout))
    } else {
        Err(format!("Error:\n{}\n{}", stdout, stderr))
    }
}

fn execute_script_command(executable: &str, args: &[String]) -> Result<String, String> {
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute script: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        Ok(format!("Success:\n{}", stdout))
    } else {
        Err(format!("Error:\n{}\n{}", stdout, stderr))
    }
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_all_tabs,
            execute_command,
            get_system_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}