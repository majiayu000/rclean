use std::io::IsTerminal;

use ratatui::style::{Color, Modifier, Style};

use crate::model::Safety;

pub fn supports_alternate_screen() -> bool {
    let term = std::env::var("TERM").unwrap_or_default();
    std::io::stdout().is_terminal() && !term.is_empty() && term != "dumb"
}

pub fn candidate_style(safety: Safety, risk_score: f32, selected: bool) -> Style {
    let mut style = Style::default().fg(base_color(safety));
    if risk_score >= 0.70 {
        style = style.fg(Color::Red);
    } else if risk_score >= 0.45 {
        style = style.fg(Color::Rgb(255, 165, 0));
    }
    if selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

fn base_color(safety: Safety) -> Color {
    match safety {
        Safety::Safe => Color::Green,
        Safety::Caution => Color::Yellow,
        Safety::Blocked => Color::DarkGray,
        Safety::ReportOnly => Color::Magenta,
        Safety::Unknown => Color::Gray,
    }
}
