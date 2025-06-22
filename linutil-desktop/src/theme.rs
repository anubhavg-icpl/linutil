use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Copy)]
pub enum Theme {
    #[default]
    Default,
    Compatible,
}

impl Theme {
    #[allow(dead_code)]
    pub fn dir_icon(&self) -> &'static str {
        match self {
            Theme::Default => "📁",
            Theme::Compatible => "[DIR]",
        }
    }

    #[allow(dead_code)]
    pub fn cmd_icon(&self) -> &'static str {
        match self {
            Theme::Default => "⚡",
            Theme::Compatible => "[CMD]",
        }
    }

    #[allow(dead_code)]
    pub fn tab_icon(&self) -> &'static str {
        match self {
            Theme::Default => "📋",
            Theme::Compatible => ">> ",
        }
    }

    #[allow(dead_code)]
    pub fn multi_select_icon(&self) -> &'static str {
        match self {
            Theme::Default => "✓",
            Theme::Compatible => "*",
        }
    }

    #[allow(dead_code)]
    pub fn next(&mut self) {
        *self = match self {
            Theme::Default => Theme::Compatible,
            Theme::Compatible => Theme::Default,
        };
    }

    #[allow(dead_code)]
    pub fn prev(&mut self) {
        self.next(); // Only two themes, so next() is the same as prev()
    }
}