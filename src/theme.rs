use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

pub struct AppTheme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub text: Color,
    pub muted: Color,
    pub highlight_bg: Color,
    pub zebra_even: Color,
    pub zebra_odd: Color,
    pub image_indicator: Color,
    pub header_style: Style,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
}

impl AppTheme {
    pub fn default_block<'a>(&self, title: &'a str) -> Block<'a> {
        Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
    }

    pub fn simple_block<'a>(&self) -> Block<'a> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
    }

    pub fn list_item_style(&self, index: usize) -> Style {
        if index % 2 == 0 {
            Style::default().bg(self.zebra_even)
        } else {
            Style::default().bg(self.zebra_odd)
        }
    }

    pub fn status_bar(&self) -> Style {
        Style::default().fg(self.status_bar_fg).bg(self.status_bar_bg)
    }

    pub fn highlight(&self) -> Style {
        Style::default().bg(self.highlight_bg)
    }

}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Gray,
            accent: Color::Yellow,
            success: Color::Green,
            error: Color::Red,
            text: Color::White,
            muted: Color::DarkGray,
            highlight_bg: Color::DarkGray,
            zebra_even: Color::Reset,
            zebra_odd: Color::Rgb(22, 22, 28),
            image_indicator: Color::Magenta,
            header_style: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            status_bar_bg: Color::Rgb(18, 18, 24),
            status_bar_fg: Color::Gray,
        }
    }
}
