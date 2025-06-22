use eframe::egui;
use linutil_core::{get_tabs, Command as LinutilCommand, TabList, ListNode, ego_tree::NodeId};
use std::process::Command;
use std::sync::{mpsc, Arc};
use std::thread;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("🐧 Linutil Desktop"),
        ..Default::default()
    };

    eframe::run_native(
        "Linutil Desktop",
        options,
        Box::new(|_cc| Ok(Box::new(LinutilApp::new()))),
    )
}

#[derive(Clone)]
pub struct ListEntry {
    pub node: Arc<ListNode>,
    pub id: NodeId,
    pub has_children: bool,
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

struct LinutilApp {
    // Core data
    tabs: TabList,
    current_tab_index: usize,
    
    // Navigation state (like TUI's visit_stack)
    visit_stack: Vec<(NodeId, usize)>, // (node_id, selection_index)
    current_items: Vec<ListEntry>,
    selected_index: usize,
    
    // Multi-selection
    multi_select: bool,
    selected_commands: Vec<Arc<ListNode>>,
    
    // UI state
    search_text: String,
    filtered_items: Vec<ListEntry>,
    
    // Command execution
    command_output: String,
    show_command_output: bool,
    executing_command: bool,
    command_tx: Option<mpsc::Sender<(String, Arc<ListNode>)>>,
    command_rx: Option<mpsc::Receiver<CommandResult>>,
    
    // Status
    loading: bool,
    error_message: String,
}

impl LinutilApp {
    fn new() -> Self {
        let mut app = Self {
            tabs: get_tabs(false), // false = don't validate, show all commands
            current_tab_index: 0,
            visit_stack: Vec::new(),
            current_items: Vec::new(),
            selected_index: 0,
            multi_select: false,
            selected_commands: Vec::new(),
            search_text: String::new(),
            filtered_items: Vec::new(),
            command_output: String::new(),
            show_command_output: false,
            executing_command: false,
            command_tx: None,
            command_rx: None,
            loading: false,
            error_message: String::new(),
        };

        // Set up command execution channel
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        
        app.command_tx = Some(cmd_tx);
        app.command_rx = Some(result_rx);

        // Spawn command execution thread
        thread::spawn(move || {
            while let Ok((_tab_name, node)) = cmd_rx.recv() {
                let result = execute_command_node(&node);
                let _ = result_tx.send(result);
            }
        });

        // Initialize navigation
        if !app.tabs.is_empty() {
            let root_id = app.tabs[0].tree.root().id();
            app.visit_stack.push((root_id, 0));
            app.update_items();
        }

        app
    }

    fn update_items(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

        let current_tab = &self.tabs[self.current_tab_index];
        let (current_node_id, _) = self.visit_stack.last().copied().unwrap_or((current_tab.tree.root().id(), 0));
        
        // Find the current node in the tree
        let current_node = current_tab.tree.get(current_node_id).unwrap();
        
        // Get children of current node
        self.current_items.clear();
        for child in current_node.children() {
            let child_value = child.value();
            let has_children = child.has_children();
            
            self.current_items.push(ListEntry {
                node: Arc::new((**child_value).clone()),
                id: child.id(),
                has_children,
            });
        }

        // Apply search filter
        self.apply_search_filter();
        
        // Ensure selected index is valid
        if self.selected_index >= self.filtered_items.len() && !self.filtered_items.is_empty() {
            self.selected_index = 0;
        }
    }

    fn apply_search_filter(&mut self) {
        if self.search_text.is_empty() {
            self.filtered_items = self.current_items.clone();
        } else {
            let search_lower = self.search_text.to_lowercase();
            self.filtered_items = self.current_items
                .iter()
                .filter(|entry| {
                    entry.node.name.to_lowercase().contains(&search_lower) ||
                    entry.node.description.to_lowercase().contains(&search_lower)
                })
                .cloned()
                .collect();
        }
    }

    fn enter_directory(&mut self) {
        if let Some(selected_entry) = self.filtered_items.get(self.selected_index) {
            if selected_entry.has_children {
                // Enter the directory
                self.visit_stack.push((selected_entry.id, self.selected_index));
                self.selected_index = 0;
                self.search_text.clear();
                self.update_items();
            }
        }
    }

    fn go_back(&mut self) {
        if self.visit_stack.len() > 1 {
            if let Some((_, previous_selection)) = self.visit_stack.pop() {
                self.selected_index = previous_selection;
                self.search_text.clear();
                self.update_items();
            }
        }
    }

    fn at_root(&self) -> bool {
        self.visit_stack.len() <= 1
    }

    fn get_breadcrumb(&self) -> String {
        if self.tabs.is_empty() {
            return "Loading...".to_string();
        }
        
        let current_tab = &self.tabs[self.current_tab_index];
        let mut path = vec![current_tab.name.clone()];
        
        for (node_id, _) in &self.visit_stack[1..] {
            if let Some(node) = current_tab.tree.get(*node_id) {
                path.push(node.value().name.clone());
            }
        }
        
        path.join(" > ")
    }

    fn execute_selected_command(&mut self) {
        if let Some(selected_entry) = self.filtered_items.get(self.selected_index) {
            if !selected_entry.has_children {
                // It's a command, execute it
                if let Some(tx) = &self.command_tx {
                    self.executing_command = true;
                    let tab_name = self.tabs[self.current_tab_index].name.clone();
                    let _ = tx.send((tab_name, selected_entry.node.clone()));
                }
            }
        }
    }

    fn toggle_multi_select(&mut self) {
        if let Some(selected_entry) = self.filtered_items.get(self.selected_index) {
            if !selected_entry.has_children && selected_entry.node.multi_select {
                if let Some(pos) = self.selected_commands.iter().position(|x| Arc::ptr_eq(x, &selected_entry.node)) {
                    self.selected_commands.remove(pos);
                } else {
                    self.selected_commands.push(selected_entry.node.clone());
                }
            }
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

    fn switch_tab(&mut self, tab_index: usize) {
        if tab_index < self.tabs.len() && tab_index != self.current_tab_index {
            self.current_tab_index = tab_index;
            // Reset navigation to root of new tab
            let root_id = self.tabs[tab_index].tree.root().id();
            self.visit_stack = vec![(root_id, 0)];
            self.selected_index = 0;
            self.search_text.clear();
            self.update_items();
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

        // Top panel with navigation and search
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🐧 Linutil Desktop");
                ui.separator();
                
                // Breadcrumb navigation
                ui.label("📍");
                ui.label(self.get_breadcrumb());
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.executing_command {
                        ui.spinner();
                        ui.label("Executing...");
                    }
                    
                    // Back button
                    if !self.at_root() {
                        if ui.button("⬅ Back").clicked() {
                            self.go_back();
                        }
                    }
                });
            });
            
            ui.horizontal(|ui| {
                ui.label("🔍 Search:");
                let response = ui.text_edit_singleline(&mut self.search_text);
                if response.changed() {
                    self.apply_search_filter();
                }
                
                if self.multi_select {
                    ui.separator();
                    ui.label(format!("Selected: {}", self.selected_commands.len()));
                    if ui.button("Execute Selected").clicked() && !self.selected_commands.is_empty() {
                        // Execute all selected commands
                        for cmd in &self.selected_commands {
                            if let Some(tx) = &self.command_tx {
                                let tab_name = self.tabs[self.current_tab_index].name.clone();
                                let _ = tx.send((tab_name, cmd.clone()));
                            }
                        }
                        self.selected_commands.clear();
                        self.multi_select = false;
                    }
                }
            });
        });

        // Left sidebar with tabs
        egui::SidePanel::left("sidebar").min_width(200.0).show(ctx, |ui| {
            ui.heading("Categories");
            ui.separator();

            let mut tab_to_switch = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, tab) in self.tabs.iter().enumerate() {
                    let selected = i == self.current_tab_index;
                    
                    if ui.selectable_label(selected, &tab.name).clicked() {
                        tab_to_switch = Some(i);
                    }
                }
            });
            
            if let Some(tab_index) = tab_to_switch {
                self.switch_tab(tab_index);
            }
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.tabs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.spinner();
                    ui.label("Loading tabs...");
                });
                return;
            }

            // Actions to perform outside the iteration
            let mut action = None;
            let at_root = self.at_root();
            
            // Show items in current directory
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Up directory option
                if !at_root {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(false, "📁 .. (Go back)").clicked() {
                            action = Some(("go_back", 0));
                        }
                    });
                    ui.separator();
                }

                // List current items
                for (i, entry) in self.filtered_items.iter().enumerate() {
                    let is_selected = i == self.selected_index;
                    let is_multi_selected = self.selected_commands.iter().any(|cmd| Arc::ptr_eq(cmd, &entry.node));
                    
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Selection indicator
                            if is_multi_selected {
                                ui.label("✅");
                            } else if is_selected {
                                ui.label("▶");
                            } else {
                                ui.label("  ");
                            }
                            
                            // Icon and name
                            let icon = if entry.has_children { "📁" } else { "⚙️" };
                            
                            if entry.has_children {
                                ui.heading(format!("{} {}", icon, entry.node.name));
                            } else {
                                ui.label(format!("{} {}", icon, entry.node.name));
                            }
                        });
                        
                        // Description
                        if !entry.node.description.is_empty() {
                            ui.label(&entry.node.description);
                        }
                        
                        // Task list info
                        if !entry.node.task_list.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("🏷️");
                                ui.small(&entry.node.task_list);
                            });
                        }
                        
                        // Action buttons
                        ui.horizontal(|ui| {
                            if entry.has_children {
                                if ui.button("📂 Enter").clicked() {
                                    action = Some(("enter", i));
                                }
                            } else {
                                if ui.button("🚀 Execute").clicked() {
                                    action = Some(("execute", i));
                                }
                                
                                if ui.button("👁️ Preview").clicked() {
                                    action = Some(("preview", i));
                                }
                                
                                if entry.node.multi_select {
                                    if ui.button("📋 Multi-Select").clicked() {
                                        action = Some(("multi_select", i));
                                    }
                                }
                            }
                        });
                    });
                    
                    ui.add_space(8.0);
                }
                
                if self.filtered_items.is_empty() && !self.search_text.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("No items match your search.");
                    });
                } else if self.current_items.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("This directory is empty.");
                    });
                }
            });
            
            // Handle actions after the iteration
            if let Some((action_type, index)) = action {
                match action_type {
                    "go_back" => self.go_back(),
                    "enter" => {
                        self.selected_index = index;
                        self.enter_directory();
                    }
                    "execute" => {
                        self.selected_index = index;
                        self.execute_selected_command();
                    }
                    "preview" => {
                        if let Some(entry) = self.filtered_items.get(index) {
                            self.command_output = format!("📋 Command Preview\n\nName: {}\nDescription: {}\nTask List: {}", 
                                                         entry.node.name, entry.node.description, entry.node.task_list);
                            self.show_command_output = true;
                        }
                    }
                    "multi_select" => {
                        self.selected_index = index;
                        self.toggle_multi_select();
                        self.multi_select = true;
                    }
                    _ => {}
                }
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

        // Error message
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

fn execute_command_node(node: &ListNode) -> CommandResult {
    match &node.command {
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