use eframe::egui;
use linutil_core::{get_tabs, Command as LinutilCommand, TabList, ListNode, ego_tree::NodeId};
use std::process::Command;
use std::sync::{mpsc, Arc};
use std::thread;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Linutil System Management Suite"),
        ..Default::default()
    };

    eframe::run_native(
        "Linutil System Management Suite",
        options,
        Box::new(|cc| {
            // Set modern dark theme
            cc.egui_ctx.set_visuals(create_modern_visuals());
            Ok(Box::new(LinutilApp::new()))
        }),
    )
}

// Modern Corporate Color Scheme
#[derive(Clone)]
struct ModernTheme {
    primary: egui::Color32,
    secondary: egui::Color32,
    accent: egui::Color32,
    success: egui::Color32,
    warning: egui::Color32,
    danger: egui::Color32,
    background: egui::Color32,
    surface: egui::Color32,
    surface_variant: egui::Color32,
    on_surface: egui::Color32,
    on_surface_variant: egui::Color32,
    border: egui::Color32,
}

impl ModernTheme {
    fn new() -> Self {
        Self {
            primary: egui::Color32::from_rgb(99, 102, 241),     // Modern blue
            secondary: egui::Color32::from_rgb(139, 92, 246),   // Purple
            accent: egui::Color32::from_rgb(34, 197, 94),       // Green
            success: egui::Color32::from_rgb(34, 197, 94),      // Green
            warning: egui::Color32::from_rgb(251, 191, 36),     // Amber
            danger: egui::Color32::from_rgb(239, 68, 68),       // Red
            background: egui::Color32::from_rgb(15, 23, 42),    // Slate 900
            surface: egui::Color32::from_rgb(30, 41, 59),       // Slate 800
            surface_variant: egui::Color32::from_rgb(51, 65, 85), // Slate 700
            on_surface: egui::Color32::from_rgb(248, 250, 252), // Slate 50
            on_surface_variant: egui::Color32::from_rgb(203, 213, 225), // Slate 300
            border: egui::Color32::from_rgb(71, 85, 105),       // Slate 600
        }
    }
}

fn create_modern_visuals() -> egui::Visuals {
    let theme = ModernTheme::new();
    let mut visuals = egui::Visuals::dark();
    
    // Modern color scheme
    visuals.window_fill = theme.background;
    visuals.panel_fill = theme.surface;
    visuals.faint_bg_color = theme.surface_variant;
    visuals.extreme_bg_color = theme.background;
    visuals.code_bg_color = theme.surface_variant;
    
    // Note: text_color, weak_text_color, and strong_text_color are methods in newer egui versions
    // We'll set text colors through widget styles instead
    
    visuals.widgets.noninteractive.bg_fill = theme.surface;
    visuals.widgets.noninteractive.weak_bg_fill = theme.surface;
    visuals.widgets.noninteractive.fg_stroke.color = theme.on_surface_variant;
    
    visuals.widgets.inactive.bg_fill = theme.surface_variant;
    visuals.widgets.inactive.weak_bg_fill = theme.surface;
    visuals.widgets.inactive.fg_stroke.color = theme.on_surface_variant;
    
    visuals.widgets.hovered.bg_fill = theme.primary.gamma_multiply(0.3);
    visuals.widgets.hovered.weak_bg_fill = theme.primary.gamma_multiply(0.2);
    visuals.widgets.hovered.fg_stroke.color = theme.on_surface;
    
    visuals.widgets.active.bg_fill = theme.primary;
    visuals.widgets.active.weak_bg_fill = theme.primary.gamma_multiply(0.8);
    visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
    
    visuals.selection.bg_fill = theme.primary.gamma_multiply(0.4);
    visuals.selection.stroke.color = theme.primary;
    
    // Modern rounded corners
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    
    // Subtle shadows and borders
    visuals.window_shadow.color = egui::Color32::from_black_alpha(50);
    visuals.popup_shadow.color = egui::Color32::from_black_alpha(30);
    
    visuals
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
    theme: ModernTheme,
    
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
    show_sidebar: bool,
    
    // Command execution
    command_output: String,
    show_command_output: bool,
    executing_command: bool,
    command_tx: Option<mpsc::Sender<(String, Arc<ListNode>)>>,
    command_rx: Option<mpsc::Receiver<CommandResult>>,
    
    // Status
    loading: bool,
    error_message: String,
    status_message: String,
}

impl LinutilApp {
    fn new() -> Self {
        let mut app = Self {
            tabs: get_tabs(false), // false = don't validate, show all commands
            current_tab_index: 0,
            theme: ModernTheme::new(),
            visit_stack: Vec::new(),
            current_items: Vec::new(),
            selected_index: 0,
            multi_select: false,
            selected_commands: Vec::new(),
            search_text: String::new(),
            filtered_items: Vec::new(),
            show_sidebar: true,
            command_output: String::new(),
            show_command_output: false,
            executing_command: false,
            command_tx: None,
            command_rx: None,
            loading: false,
            error_message: String::new(),
            status_message: "Ready".to_string(),
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
            app.status_message = format!("Loaded {} categories with {} total utilities", 
                                       app.tabs.len(), 
                                       app.tabs.iter().map(|t| t.tree.root().descendants().count() - 1).sum::<usize>());
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
        let selected_info = self.filtered_items.get(self.selected_index)
            .map(|entry| (entry.id, entry.has_children, entry.node.name.clone()));
        
        if let Some((entry_id, has_children, node_name)) = selected_info {
            if has_children {
                // Enter the directory
                self.visit_stack.push((entry_id, self.selected_index));
                self.selected_index = 0;
                self.search_text.clear();
                self.update_items();
                self.status_message = format!("Navigated to {}", node_name);
            }
        }
    }

    fn go_back(&mut self) {
        if self.visit_stack.len() > 1 {
            if let Some((_, previous_selection)) = self.visit_stack.pop() {
                self.selected_index = previous_selection;
                self.search_text.clear();
                self.update_items();
                self.status_message = "Navigated back".to_string();
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
        
        path.join(" › ")
    }

    fn execute_selected_command(&mut self) {
        if let Some(selected_entry) = self.filtered_items.get(self.selected_index) {
            if !selected_entry.has_children {
                // It's a command, execute it
                if let Some(tx) = &self.command_tx {
                    self.executing_command = true;
                    let tab_name = self.tabs[self.current_tab_index].name.clone();
                    let _ = tx.send((tab_name, selected_entry.node.clone()));
                    self.status_message = format!("Executing: {}", selected_entry.node.name);
                }
            }
        }
    }

    fn toggle_multi_select(&mut self) {
        if let Some(selected_entry) = self.filtered_items.get(self.selected_index) {
            if !selected_entry.has_children && selected_entry.node.multi_select {
                if let Some(pos) = self.selected_commands.iter().position(|x| Arc::ptr_eq(x, &selected_entry.node)) {
                    self.selected_commands.remove(pos);
                    self.status_message = format!("Removed {} from selection", selected_entry.node.name);
                } else {
                    self.selected_commands.push(selected_entry.node.clone());
                    self.status_message = format!("Added {} to selection", selected_entry.node.name);
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
                self.status_message = if result.success { 
                    "Command completed successfully".to_string() 
                } else { 
                    "Command failed".to_string() 
                };
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
            self.status_message = format!("Switched to {}", self.tabs[tab_index].name);
        }
    }

    fn render_modern_button(&self, ui: &mut egui::Ui, text: &str, icon: &str, color: egui::Color32) -> egui::Response {
        let button_height = 32.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), button_height),
            egui::Sense::click()
        );

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let bg_color = if response.hovered() {
                color.gamma_multiply(1.2)
            } else {
                color
            };

            ui.painter().rect_filled(rect, visuals.rounding, bg_color);
            
            let text_color = if response.hovered() {
                egui::Color32::WHITE
            } else {
                self.theme.on_surface
            };

            ui.painter().text(
                rect.left_center() + egui::vec2(12.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("{} {}", icon, text),
                egui::FontId::proportional(14.0),
                text_color,
            );
        }

        response
    }

    fn render_category_card(&self, ui: &mut egui::Ui, entry: &ListEntry, _index: usize) -> Option<String> {
        let mut action = None;
        
        let is_multi_selected = self.selected_commands.iter().any(|cmd| Arc::ptr_eq(cmd, &entry.node));
        
        // Card styling
        let card_color = if is_multi_selected {
            self.theme.primary.gamma_multiply(0.3)
        } else {
            self.theme.surface
        };

        let response = egui::Frame::none()
            .fill(card_color)
            .rounding(12.0)
            .inner_margin(egui::Margin::same(16.0))
            .stroke(egui::Stroke::new(1.0, self.theme.border))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Header with icon and title
                    ui.horizontal(|ui| {
                        let icon = if entry.has_children { "📁" } else { "⚙️" };
                        let status_icon = if is_multi_selected { " ✅" } else { "" };
                        
                        ui.label(egui::RichText::new(format!("{} {}{}", icon, entry.node.name, status_icon))
                                .size(16.0)
                                .strong()
                                .color(self.theme.on_surface));
                                
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !entry.node.task_list.is_empty() {
                                ui.label(egui::RichText::new(&entry.node.task_list)
                                        .size(10.0)
                                        .background_color(self.theme.secondary.gamma_multiply(0.3))
                                        .color(self.theme.on_surface_variant));
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Description
                    if !entry.node.description.is_empty() {
                        ui.label(egui::RichText::new(&entry.node.description)
                                .size(13.0)
                                .color(self.theme.on_surface_variant));
                        ui.add_space(12.0);
                    }

                    // Action buttons
                    ui.horizontal(|ui| {
                        if entry.has_children {
                            if self.render_modern_button(ui, "Open", "📂", self.theme.primary).clicked() {
                                action = Some("enter".to_string());
                            }
                        } else {
                            if self.render_modern_button(ui, "Execute", "▶️", self.theme.success).clicked() {
                                action = Some("execute".to_string());
                            }
                            
                            ui.add_space(8.0);
                            
                            if self.render_modern_button(ui, "Preview", "👁️", self.theme.secondary).clicked() {
                                action = Some("preview".to_string());
                            }
                            
                            if entry.node.multi_select {
                                ui.add_space(8.0);
                                let multi_text = if is_multi_selected { "Deselect" } else { "Select" };
                                if self.render_modern_button(ui, multi_text, "☑️", self.theme.accent).clicked() {
                                    action = Some("multi_select".to_string());
                                }
                            }
                        }
                    });
                })
            })
            .response;
        
        // Add hover effect to entire card
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        action
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

        // Modern top bar
        egui::TopBottomPanel::top("top_panel")
            .min_height(64.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    // Logo and title
                    ui.label(egui::RichText::new("🐧 Linutil")
                            .size(24.0)
                            .strong()
                            .color(self.theme.primary));
                    ui.label(egui::RichText::new("System Management Suite")
                            .size(16.0)
                            .color(self.theme.on_surface_variant));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Toggle sidebar button
                        if ui.button(if self.show_sidebar { "◀" } else { "▶" }).clicked() {
                            self.show_sidebar = !self.show_sidebar;
                        }
                        
                        ui.add_space(16.0);
                        
                        // Execution status
                        if self.executing_command {
                            ui.spinner();
                            ui.label(egui::RichText::new("Executing...")
                                    .color(self.theme.warning));
                        }
                        
                        // Multi-select indicator
                        if !self.selected_commands.is_empty() {
                            ui.label(egui::RichText::new(format!("{} selected", self.selected_commands.len()))
                                    .background_color(self.theme.accent.gamma_multiply(0.3))
                                    .color(self.theme.on_surface));
                            
                            if ui.button("Execute All").clicked() {
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
                
                ui.add_space(8.0);
                
                // Navigation bar
                ui.horizontal(|ui| {
                    // Breadcrumb
                    ui.label(egui::RichText::new("📍")
                            .color(self.theme.accent));
                    ui.label(egui::RichText::new(self.get_breadcrumb())
                            .size(14.0)
                            .color(self.theme.on_surface_variant));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Back button
                        if !self.at_root() {
                            if ui.button("⬅ Back").clicked() {
                                self.go_back();
                            }
                        }
                        
                        ui.add_space(16.0);
                        
                        // Search
                        ui.label("🔍");
                        let search_response = ui.add_sized([200.0, 24.0], 
                            egui::TextEdit::singleline(&mut self.search_text)
                                .hint_text("Search utilities..."));
                        if search_response.changed() {
                            self.apply_search_filter();
                        }
                    });
                });
                ui.add_space(8.0);
            });

        // Status bar
        egui::TopBottomPanel::bottom("status_panel")
            .min_height(32.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.status_message)
                            .size(12.0)
                            .color(self.theme.on_surface_variant));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{} items", self.filtered_items.len()))
                                .size(12.0)
                                .color(self.theme.on_surface_variant));
                    });
                });
                ui.add_space(4.0);
            });

        // Modern sidebar
        if self.show_sidebar {
            egui::SidePanel::left("sidebar")
                .min_width(280.0)
                .max_width(350.0)
                .show(ctx, |ui| {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Categories")
                            .size(18.0)
                            .strong()
                            .color(self.theme.on_surface));
                    ui.add_space(8.0);

                    let mut tab_to_switch = None;
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, tab) in self.tabs.iter().enumerate() {
                            let selected = i == self.current_tab_index;
                            
                            let response = ui.selectable_label(selected, 
                                egui::RichText::new(&tab.name)
                                    .size(14.0)
                                    .color(if selected { egui::Color32::WHITE } else { self.theme.on_surface }));
                            
                            if response.clicked() {
                                tab_to_switch = Some(i);
                            }
                            
                            ui.add_space(4.0);
                        }
                    });
                    
                    if let Some(tab_index) = tab_to_switch {
                        self.switch_tab(tab_index);
                    }
                });
        }

        // Main content with modern grid layout
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.tabs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Loading system utilities...")
                                .size(16.0)
                                .color(self.theme.on_surface_variant));
                    });
                });
                return;
            }

            let mut action = None;
            let mut action_index = 0;
            
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_space(16.0);
                    
                    // Up directory card
                    if !self.at_root() {
                        egui::Frame::none()
                            .fill(self.theme.surface_variant)
                            .rounding(12.0)
                            .inner_margin(16.0)
                            .stroke(egui::Stroke::new(1.0, self.theme.border))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("📁 .. Go Back").clicked() {
                                        action = Some("go_back".to_string());
                                    }
                                });
                            });
                        ui.add_space(16.0);
                    }

                    // Modern grid layout
                    let available_width = ui.available_width();
                    let card_width = 350.0;
                    let spacing = 16.0;
                    let cols = ((available_width + spacing) / (card_width + spacing)).floor() as usize;
                    let cols = cols.max(1);
                    
                    ui.columns(cols, |columns| {
                        for (i, entry) in self.filtered_items.iter().enumerate() {
                            let col = i % cols;
                            if let Some(entry_action) = self.render_category_card(&mut columns[col], entry, i) {
                                action = Some(entry_action);
                                action_index = i;
                            }
                            columns[col].add_space(16.0);
                        }
                    });

                    if self.filtered_items.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("🔍")
                                        .size(48.0)
                                        .color(self.theme.on_surface_variant));
                                ui.add_space(16.0);
                                let message = if !self.search_text.is_empty() {
                                    "No utilities match your search"
                                } else {
                                    "This category is empty"
                                };
                                ui.label(egui::RichText::new(message)
                                        .size(16.0)
                                        .color(self.theme.on_surface_variant));
                            });
                        });
                    }
                });

            // Handle actions
            if let Some(action_type) = action {
                match action_type.as_str() {
                    "go_back" => self.go_back(),
                    "enter" => {
                        self.selected_index = action_index;
                        self.enter_directory();
                    }
                    "execute" => {
                        self.selected_index = action_index;
                        self.execute_selected_command();
                    }
                    "preview" => {
                        if let Some(entry) = self.filtered_items.get(action_index) {
                            self.command_output = format!("📋 Command Preview\n\nName: {}\nDescription: {}\nTask List: {}", 
                                                         entry.node.name, entry.node.description, entry.node.task_list);
                            self.show_command_output = true;
                        }
                    }
                    "multi_select" => {
                        self.selected_index = action_index;
                        self.toggle_multi_select();
                        self.multi_select = true;
                    }
                    _ => {}
                }
            }
        });

        // Modern command output window
        if self.show_command_output {
            egui::Window::new("Command Output")
                .default_width(700.0)
                .default_height(500.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut self.command_output.as_str())
                               .font(egui::TextStyle::Monospace)
                               .desired_rows(20)
                               .desired_width(f32::INFINITY));
                    });
                    
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("📋 Copy").clicked() {
                            ui.output_mut(|o| o.copied_text = self.command_output.clone());
                            self.status_message = "Output copied to clipboard".to_string();
                        }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕ Close").clicked() {
                                self.show_command_output = false;
                            }
                        });
                    });
                });
        }

        // Error dialog
        if !self.error_message.is_empty() {
            egui::Window::new("⚠️ Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(&self.error_message);
                    ui.add_space(12.0);
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