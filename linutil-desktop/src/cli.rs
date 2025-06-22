use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
pub struct Args {
    /// Path to the configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Skip confirmation prompt before executing commands
    #[arg(short = 'y', long)]
    pub skip_confirmation: bool,

    /// Show all available options, disregarding compatibility checks (UNSAFE)
    #[arg(short = 'u', long)]
    pub override_validation: bool,

    /// Bypass the terminal size limit
    #[arg(short = 's', long)]
    pub size_bypass: bool,

    /// Enable mouse interaction
    #[arg(short = 'm', long)]
    pub mouse: bool,

    /// Bypass root user check
    #[arg(short = 'r', long)]
    pub bypass_root: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            config: None,
            skip_confirmation: false,
            override_validation: true, // Default to true for desktop app to avoid loops
            size_bypass: true,
            mouse: true,
            bypass_root: true,
        }
    }
}