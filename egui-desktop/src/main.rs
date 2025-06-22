use eframe::egui;
use linutil_core::{get_tabs, Command as LinutilCommand, Tab, ListNode, ego_tree::{NodeId, Tree}};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::rc::Rc;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("🐧 Linutil Desktop (eGUI)"),
        ..Default::default()
    };

    eframe::run_native(
        "Linutil Desktop",
        options,
        Box::new(|_cc| Ok(Box::new(LinutilApp::new()))),
    )
}

#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub name: String,
    pub description: String,
    pub command_type: String,
    pub task_list: String,
    pub multi_select: bool,
    pub has_children: bool,
}

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub name: String,
    pub entries: Vec<EntryInfo>,
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

struct LinutilApp {
    tabs: Vec<TabInfo>,
    selected_tab: Option<usize>,
    search_text: String,
    filtered_entries: Vec<EntryInfo>,
    current_tab_entries: Vec<EntryInfo>,
    loading: bool,
    error_message: String,
    command_output: String,
    show_command_output: bool,
    executing_command: bool,
    command_tx: Option<mpsc::Sender<(String, String)>>,
    command_rx: Option<mpsc::Receiver<CommandResult>>,
}

impl LinutilApp {
    fn new() -> Self {
        let mut app = Self {
            tabs: Vec::new(),
            selected_tab: None,
            search_text: String::new(),
            filtered_entries: Vec::new(),
            current_tab_entries: Vec::new(),
            loading: true,
            error_message: String::new(),
            command_output: String::new(),
            show_command_output: false,
            executing_command: false,
            command_tx: None,
            command_rx: None,
        };

        // Set up command execution channel
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        
        app.command_tx = Some(cmd_tx);
        app.command_rx = Some(result_rx);

        // Spawn command execution thread
        thread::spawn(move || {
            while let Ok((tab_name, entry_name)) = cmd_rx.recv() {
                let result = execute_command(&tab_name, &entry_name);
                let _ = result_tx.send(result);
            }
        });

        // Load tabs in background
        app.load_tabs();
        app
    }

    fn load_tabs(&mut self) {
        println!("Loading tabs from core...");
        
        // Load tabs from core library
        let core_tabs = get_tabs(false); // false = don't validate, show all
        let mut tabs = Vec::new();

        for core_tab in core_tabs.iter() {
            let mut tab_info = TabInfo {
                name: core_tab.name.clone(),
                entries: Vec::new(),
            };

            // Convert core entries to our format
            for node in core_tab.tree.root().descendants() {
                let node_value = node.value();
                if node_value.name != "root" {
                    let command_type = match &node_value.command {
                        LinutilCommand::Raw(_) => "raw",
                        LinutilCommand::LocalFile { .. } => "script",
                        LinutilCommand::None => "directory",
                    };

                    let entry = EntryInfo {
                        name: node_value.name.clone(),
                        description: node_value.description.clone(),
                        command_type: command_type.to_string(),
                        task_list: node_value.task_list.clone(),
                        multi_select: node_value.multi_select,
                        has_children: node.has_children(),
                    };

                    tab_info.entries.push(entry);
                }
            }

            tabs.push(tab_info);
        }

        self.tabs = tabs;
        self.loading = false;
        
        // Select first tab by default
        if !self.tabs.is_empty() {
            self.selected_tab = Some(0);
            self.current_tab_entries = self.tabs[0].entries.clone();
            self.update_filtered_entries();
        }

        println!("Loaded {} tabs successfully", self.tabs.len());
    }

    fn update_filtered_entries(&mut self) {
        if self.search_text.is_empty() {
            self.filtered_entries = self.current_tab_entries.clone();
        } else {
            self.filtered_entries = self.current_tab_entries
                .iter()
                .filter(|entry| {
                    entry.name.to_lowercase().contains(&self.search_text.to_lowercase()) ||
                    entry.description.to_lowercase().contains(&self.search_text.to_lowercase())
                })
                .cloned()
                .collect();
        }
    }

    fn execute_command_async(&mut self, tab_name: &str, entry_name: &str) {
        if let Some(tx) = &self.command_tx {
            self.executing_command = true;
            let _ = tx.send((tab_name.to_string(), entry_name.to_string()));
        }
    }

    fn check_command_result(&mut self) {
        if let Some(rx) = &self.command_rx {
            if let Ok(result) = rx.try_recv() {
                self.executing_command = false;
                self.command_output = if result.success {
                    format!("✅ Command executed successfully!\n\n{}", result.output)
                } else {
                    format!("❌ Command failed!\n\n{}\n\nError: {}", 
                           result.output, result.error.unwrap_or_default())
                };
                self.show_command_output = true;
            }
        }
    }
}

impl eframe::App for LinutilApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for command execution results
        self.check_command_result();

        // Force repaint for loading states
        if self.loading || self.executing_command {
            ctx.request_repaint();
        }

        // Top panel with title and search
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🐧 Linutil Desktop");
                ui.add_space(20.0);
                
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut self.search_text);
                if response.changed() {
                    self.update_filtered_entries();
                }
                
                if self.executing_command {
                    ui.spinner();
                    ui.label("Executing command...");
                }
            });
        });

        // Left sidebar with tabs
        egui::SidePanel::left("sidebar").min_width(250.0).show(ctx, |ui| {
            ui.heading("Categories");
            ui.separator();

            if self.loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading tabs...");
                });
            } else if self.tabs.is_empty() {
                ui.label("No tabs loaded");
            } else {
                let mut selected_tab_changed = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, tab) in self.tabs.iter().enumerate() {
                        let selected = self.selected_tab == Some(i);
                        
                        if ui.selectable_label(selected, &tab.name).clicked() {
                            selected_tab_changed = Some((i, tab.entries.clone()));
                        }
                        
                        if selected {
                            ui.label(format!("({} utilities)", tab.entries.len()));
                        }
                    }
                });
                
                if let Some((tab_idx, entries)) = selected_tab_changed {
                    self.selected_tab = Some(tab_idx);
                    self.current_tab_entries = entries;
                    self.update_filtered_entries();
                }
            }
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.loading {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.spinner();
                    ui.label("Loading application data...");
                });
            } else if let Some(tab_idx) = self.selected_tab {
                let tab_name = self.tabs[tab_idx].name.clone();
                ui.heading(format!("📋 {}", tab_name));
                ui.separator();

                if self.filtered_entries.is_empty() {
                    ui.label("No utilities found matching your search.");
                } else {
                    let filtered_entries = self.filtered_entries.clone();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.columns(2, |columns| {
                            let entries_per_column = (filtered_entries.len() + 1) / 2;
                            
                            for (col_idx, column) in columns.iter_mut().enumerate() {
                                let start_idx = col_idx * entries_per_column;
                                let end_idx = ((col_idx + 1) * entries_per_column).min(filtered_entries.len());
                                
                                for entry in &filtered_entries[start_idx..end_idx] {
                                    // Skip directories for execution
                                    if entry.command_type == "directory" {
                                        continue;
                                    }

                                    column.group(|ui| {
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.strong(&entry.name);
                                                ui.label(format!("[{}]", entry.command_type));
                                            });
                                            
                                            ui.label(&entry.description);
                                            
                                            if !entry.task_list.is_empty() {
                                                ui.horizontal(|ui| {
                                                    ui.label("🏷️");
                                                    ui.small(&entry.task_list);
                                                });
                                            }
                                            
                                            let entry_name = entry.name.clone();
                                            let entry_desc = entry.description.clone();
                                            let entry_type = entry.command_type.clone();
                                            let tab_name_clone = tab_name.clone();
                                            
                                            ui.horizontal(|ui| {
                                                if ui.button("🚀 Execute").clicked() {
                                                    self.execute_command_async(&tab_name_clone, &entry_name);
                                                }
                                                
                                                if ui.button("👁️ Preview").clicked() {
                                                    // For now, just show the description
                                                    self.command_output = format!("📋 Command Preview\n\nName: {}\nType: {}\nDescription: {}", 
                                                                                 entry_name, entry_type, entry_desc);
                                                    self.show_command_output = true;
                                                }
                                            });
                                        });
                                    });
                                    column.add_space(10.0);
                                }
                            }
                        });
                    });
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label("Select a tab to view utilities");
                });
            }
        });

        // Command output window
        if self.show_command_output {
            egui::Window::new("Command Output")
                .default_width(600.0)
                .default_height(400.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.monospace(&self.command_output);
                    });
                    
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_command_output = false;
                        }
                        
                        if ui.button("Copy to Clipboard").clicked() {
                            ui.output_mut(|o| o.copied_text = self.command_output.clone());
                        }
                    });
                });
        }

        if !self.error_message.is_empty() {
            egui::Window::new("Error")
                .show(ctx, |ui| {
                    ui.label(&self.error_message);
                    if ui.button("OK").clicked() {
                        self.error_message.clear();
                    }
                });
        }
    }
}

fn execute_command(tab_name: &str, entry_name: &str) -> CommandResult {
    println!("Executing command: {} from tab: {}", entry_name, tab_name);
    
    // Load tabs fresh to find the command
    let tabs = get_tabs(false);
    
    // Find the tab and command
    let tab = tabs.iter().find(|t| t.name == tab_name);
    if tab.is_none() {
        return CommandResult {
            success: false,
            output: "Tab not found".to_string(),
            error: Some("Could not find the specified tab".to_string()),
        };
    }
    
    let tab = tab.unwrap();
    
    // Find the command in the tab
    let command_node = tab.tree.root().descendants()
        .find(|node| {
            let node_value = node.value();
            node_value.name == entry_name && !node.has_children()
        });
        
    if command_node.is_none() {
        return CommandResult {
            success: false,
            output: "Command not found".to_string(),
            error: Some("Could not find the specified command".to_string()),
        };
    }
    
    let node_value = command_node.unwrap().value();
    
    match &node_value.command {
        LinutilCommand::Raw(cmd) => {
            execute_raw_command(cmd)
        },
        LinutilCommand::LocalFile { executable, args, file } => {
            execute_script_file(executable, args, file)
        },
        LinutilCommand::None => {
            CommandResult {
                success: false,
                output: "Cannot execute directory".to_string(),
                error: Some("This is a directory, not an executable command".to_string()),
            }
        }
    }
}

fn execute_raw_command(cmd: &str) -> CommandResult {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("DEBIAN_FRONTEND", "noninteractive")
        .output();
        
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            let success = output.status.success();
            let result_output = if stdout.is_empty() && !stderr.is_empty() {
                stderr.clone()
            } else if !stdout.is_empty() {
                stdout
            } else {
                "Command executed successfully".to_string()
            };
            
            CommandResult {
                success,
                output: result_output,
                error: if success { None } else { Some(stderr) },
            }
        },
        Err(e) => {
            CommandResult {
                success: false,
                output: format!("Failed to execute command: {}", e),
                error: Some(e.to_string()),
            }
        }
    }
}

fn execute_script_file(executable: &str, args: &[String], file: &std::path::PathBuf) -> CommandResult {
    let script_dir = file.parent().unwrap_or_else(|| std::path::Path::new("."));
    
    let output = Command::new(executable)
        .args(args)
        .current_dir(script_dir)
        .env("DEBIAN_FRONTEND", "noninteractive")
        .output();
        
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            let success = output.status.success();
            let result_output = if stdout.is_empty() && !stderr.is_empty() {
                stderr.clone()
            } else if !stdout.is_empty() {
                stdout
            } else {
                "Script executed successfully".to_string()
            };
            
            CommandResult {
                success,
                output: result_output,
                error: if success { None } else { Some(stderr) },
            }
        },
        Err(e) => {
            CommandResult {
                success: false,
                output: format!("Failed to execute script: {}", e),
                error: Some(e.to_string()),
            }
        }
    }
}