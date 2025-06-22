# Linutil Data Flow: TUI to Tauri Desktop Integration

## Overview
This document explains how data flows from the original TUI (Terminal User Interface) codebase to the new Tauri Desktop application.

## Architecture Components

### 1. Core Library (`/core`)
**Location**: `/home/mranv/Desktop/linutil/core/`

The core library serves as the shared foundation for both TUI and desktop applications:

- **`lib.rs`**: Main entry point, exports `get_tabs()` function and data structures
- **`inner.rs`**: Contains the core logic for loading tabs and commands from filesystem
- **`config.rs`**: Configuration handling and validation

**Key Data Structures**:
```rust
pub struct Tab {
    pub name: String,
    pub tree: Tree<Rc<ListNode>>,
}

pub struct ListNode {
    pub name: String,
    pub description: String,
    pub command: Command,
    pub task_list: String,
    pub multi_select: bool,
}

pub enum Command {
    Raw(String),                    // Shell commands
    LocalFile { ... },              // Script files
    None,                           // Directories
}
```

### 2. TUI Application (`/tui`)
**Location**: `/home/mranv/Desktop/linutil/tui/`

The original terminal interface:

- **`main.rs`**: Entry point, sets up terminal interface using `ratatui`
- **`state.rs`**: Application state management
- **`cli.rs`**: Command-line argument parsing

**Data Flow in TUI**:
1. `main()` → `AppState::new()` → calls `get_tabs()`
2. User interacts with terminal interface
3. Commands executed directly via shell

### 3. Tauri Desktop (`/linutil-desktop`)
**Location**: `/home/mranv/Desktop/linutil/linutil-desktop/`

The desktop GUI application:

- **`main.rs`**: Tauri backend entry point, command handlers
- **`core_integration.rs`**: Bridge between core library and Tauri
- **Frontend**: HTML/CSS/JavaScript in `/dist/index.html`

## Data Flow from TUI to Tauri

### 1. Shared Core Library
Both TUI and Tauri use the same core library:

```
┌─────────────┐    ┌─────────────────┐    ┌──────────────┐
│     TUI     │───▶│   Core Library  │◀───│    Tauri     │
│   (main.rs) │    │   (get_tabs())  │    │ (main.rs)    │
└─────────────┘    └─────────────────┘    └──────────────┘
                           │
                           ▼
                   ┌─────────────────┐
                   │  Tab Data Files │
                   │ (/core/tabs/)   │
                   └─────────────────┘
```

### 2. Data Transformation Layer
The `core_integration.rs` transforms core data structures into Tauri-compatible formats:

**Core → Tauri Transformation**:
```rust
// Core format (used by TUI)
Tab { name, tree: Tree<ListNode> }

// Tauri format (used by desktop)
TabInfo { name, entries: Vec<EntryInfo> }
```

### 3. Frontend Integration
The desktop frontend receives data through Tauri commands:

```javascript
// Frontend calls Tauri backend
currentTabs = await invoke('get_all_tabs');

// Backend processes request
#[tauri::command]
fn get_all_tabs() -> Result<Vec<TabInfo>, String> {
    load_tabs_with_core(&config)
}
```

## Command Execution Flow

### TUI Command Execution:
1. User selects command in terminal
2. Direct shell execution via `std::process::Command`
3. Output displayed in terminal

### Tauri Command Execution:
1. User clicks command in GUI
2. Frontend calls `invoke('execute_command', ...)`
3. Backend receives call, finds command in core data
4. Executes via `execute_command_with_core()`
5. Returns structured result to frontend
6. Frontend displays output in modal

## Key Files and Their Roles

| File | Role | Data Flow |
|------|------|-----------|
| `/core/tabs/` | Tab data storage | TOML files → Core structs |
| `/core/src/inner.rs` | Data loading logic | Files → `Tab` structs |
| `/tui/src/main.rs` | TUI entry point | Core → Terminal UI |
| `/linutil-desktop/src/core_integration.rs` | Data bridge | Core → Tauri format |
| `/linutil-desktop/src/main.rs` | Tauri backend | Tauri commands → Core calls |
| `/dist/index.html` | Desktop frontend | User interaction → Tauri calls |

## Benefits of This Architecture

1. **Code Reuse**: Both TUI and desktop share the same command logic
2. **Consistency**: Same commands available in both interfaces
3. **Maintainability**: Single source of truth for command definitions
4. **Extensibility**: Easy to add new interfaces (web, mobile, etc.)

## Current Implementation Status

✅ **Working Components**:
- Core library integration
- Tauri command handlers
- Frontend-backend communication
- Command execution pipeline

🔧 **Known Issues**:
- Initialization may hang on some systems (fixed with improved error handling)
- Loading screen improvements added for better user feedback

## Usage Example

To see the data flow in action:

1. **TUI**: Run `cargo run` in `/tui` directory
2. **Desktop**: Run `cargo run --release` in `/linutil-desktop` directory
3. **Both interfaces** access the same underlying command data from `/core/tabs/`

The desktop application now includes improved initialization with progress feedback and error handling to prevent hanging during startup.