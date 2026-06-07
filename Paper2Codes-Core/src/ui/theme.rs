use ratatui::style::{Color, Modifier, Style};

/// Rust orange color theme
#[derive(Clone)]
pub struct Theme {
    pub rust_orange: Color,
    pub rust_orange_dark: Color,
    pub rust_orange_light: Color,
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            // Rust orange colors (RGB: #CE412B, #A8321F, #E85D3F)
            rust_orange: Color::Rgb(206, 65, 43),
            rust_orange_dark: Color::Rgb(168, 50, 31),
            rust_orange_light: Color::Rgb(232, 93, 63),
            background: Color::Rgb(28, 28, 28), // Dark background
            foreground: Color::Rgb(240, 240, 240),
            border: Color::Rgb(100, 100, 100),
            success: Color::Rgb(46, 204, 113),
            error: Color::Rgb(231, 76, 60),
            warning: Color::Rgb(241, 196, 15),
            info: Color::Rgb(52, 152, 219),
        }
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.rust_orange)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn highlight_style(&self) -> Style {
        Style::default()
            .fg(self.rust_orange_light)
            .bg(self.rust_orange_dark)
            .add_modifier(Modifier::BOLD)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn info_style(&self) -> Style {
        Style::default().fg(self.info)
    }

    pub fn code_style(&self) -> Style {
        Style::default().fg(self.foreground)
    }

    pub fn muted_style(&self) -> Style {
        Style::default().fg(Color::DarkGray)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
