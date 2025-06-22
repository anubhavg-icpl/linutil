use crate::{AppConfig, TabInfo, EntryInfo};
use linutil_core::{get_tabs, Command as LinutilCommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

static TABS_CACHE: Mutex<Option<Vec<TabInfo>>> = Mutex::new(None);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}

/// Load tabs using the core library with proper validation handling
pub fn load_tabs_with_core(app_config: &AppConfig) -> Result<Vec<TabInfo>, String> {
    let mut cache = TABS_CACHE.lock().unwrap();
    
    // Check if we have cached tabs
    if let Some(ref cached_tabs) = *cache {
        return Ok(cached_tabs.clone());
    }
    
    // The validate parameter: true = apply validation, false = skip validation (show all)
    // For desktop app, we want to skip validation to avoid compatibility issues
    let validate = !app_config.override_validation;
    
    let tabs = get_tabs(validate);
    let mut result = Vec::new();
    
    for tab in tabs.iter() {
        let mut tab_info = TabInfo {
            name: tab.name.clone(),
            entries: Vec::new(),
        };
        
        // Get all entries from the tree, excluding root
        for node in tab.tree.root().descendants() {
            let node_value = node.value();
            if node_value.name != "root" {
                let (command_type, command_content) = match &node_value.command {
                    LinutilCommand::Raw(cmd) => {
                        ("raw".to_string(), cmd.clone())
                    },
                    LinutilCommand::LocalFile { executable, args, file } => {
                        // Store the actual file path and execution details
                        let file_str = file.to_string_lossy().to_string();
                        ("script".to_string(), format!("{}|{}|{}", executable, args.join(" "), file_str))
                    },
                    LinutilCommand::None => {
                        ("directory".to_string(), String::new())
                    },
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
    
    // Cache the result
    *cache = Some(result.clone());
    Ok(result)
}

/// Execute a command using the core library's logic
pub fn execute_command_with_core(tab_name: &str, entry_name: &str, _app_config: &AppConfig) -> Result<CommandExecutionResult, String> {
    // Load tabs fresh to avoid thread safety issues
    let tabs = get_tabs(false); // false = skip validation for desktop
    
    // Find the tab and command
    let tab = tabs.iter()
        .find(|t| t.name == tab_name)
        .ok_or("Tab not found")?;
    
    // Find the command in the tab
    let command_node = tab.tree.root().descendants()
        .find(|node| {
            let node_value = node.value();
            node_value.name == entry_name && !node.has_children()
        })
        .ok_or("Command not found")?;
    
    let node_value = command_node.value();
    
    match &node_value.command {
        LinutilCommand::Raw(cmd) => {
            execute_raw_command(cmd)
        },
        LinutilCommand::LocalFile { executable, args, file } => {
            execute_script_file(executable, args, file)
        },
        LinutilCommand::None => {
            Err("Cannot execute directory".to_string())
        }
    }
}

/// Execute a raw command with proper environment setup
fn execute_raw_command(cmd: &str) -> Result<CommandExecutionResult, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("DEBIAN_FRONTEND", "noninteractive") // Prevent interactive prompts
        .env("NEEDRESTART_MODE", "a") // Auto restart services
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    let success = output.status.success();
    let exit_code = output.status.code();
    
    let result_output = if stdout.is_empty() && !stderr.is_empty() {
        stderr.clone()
    } else if !stdout.is_empty() {
        stdout
    } else {
        "Command executed successfully".to_string()
    };
    
    Ok(CommandExecutionResult {
        success,
        output: result_output,
        error: if success { None } else { Some(stderr) },
        exit_code,
    })
}

/// Execute a script file with proper working directory and environment
fn execute_script_file(executable: &str, args: &[String], file: &PathBuf) -> Result<CommandExecutionResult, String> {
    // Get the script directory to set as working directory
    let script_dir = file.parent()
        .ok_or("Could not determine script directory")?;
    
    // Prepare arguments - the script path should already be included in args
    let output = Command::new(executable)
        .args(args)
        .current_dir(script_dir) // Set working directory to script location
        .env("DEBIAN_FRONTEND", "noninteractive")
        .env("NEEDRESTART_MODE", "a")
        .env("PATH", std::env::var("PATH").unwrap_or_default()) // Preserve PATH
        .output()
        .map_err(|e| format!("Failed to execute script: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    let success = output.status.success();
    let exit_code = output.status.code();
    
    let result_output = if stdout.is_empty() && !stderr.is_empty() {
        stderr.clone()
    } else if !stdout.is_empty() {
        stdout
    } else {
        "Script executed successfully".to_string()
    };
    
    Ok(CommandExecutionResult {
        success,
        output: result_output,
        error: if success { None } else { Some(stderr) },
        exit_code,
    })
}

/// Get command preview with actual script content
pub fn get_command_preview_with_core(tab_name: &str, entry_name: &str) -> Result<String, String> {
    // Load tabs fresh to avoid thread safety issues
    let tabs = get_tabs(false); // false = skip validation for desktop
    
    // Find the tab and command
    let tab = tabs.iter()
        .find(|t| t.name == tab_name)
        .ok_or("Tab not found")?;
    
    // Find the command in the tab
    let command_node = tab.tree.root().descendants()
        .find(|node| {
            let node_value = node.value();
            node_value.name == entry_name && !node.has_children()
        })
        .ok_or("Command not found")?;
    
    let node_value = command_node.value();
    
    match &node_value.command {
        LinutilCommand::Raw(cmd) => {
            Ok(format!("Raw Command:\n{}\n\nDescription:\n{}", cmd, node_value.description))
        },
        LinutilCommand::LocalFile { executable, args, file } => {
            // Try to read the actual script content
            let script_content = std::fs::read_to_string(file)
                .unwrap_or_else(|_| format!("Could not read script file: {}", file.display()));
            
            let execution_info = format!("Executable: {}\nArguments: {}\nScript File: {}", 
                executable, args.join(" "), file.display());
            
            Ok(format!("Script Preview:\n{}\n\nExecution Info:\n{}\n\nDescription:\n{}", 
                script_content, execution_info, node_value.description))
        },
        LinutilCommand::None => {
            Ok(format!("Directory: {}\n\nDescription:\n{}", entry_name, node_value.description))
        }
    }
}

/// Clear the tabs cache
pub fn clear_tabs_cache() {
    let mut cache = TABS_CACHE.lock().unwrap();
    *cache = None;
}

/// Get system information
pub fn get_system_info() -> HashMap<String, String> {
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
    
    // Get architecture
    if let Ok(output) = Command::new("uname").arg("-m").output() {
        info.insert("architecture".to_string(), String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    
    info
}