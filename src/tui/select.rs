use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::clean::SelectedCandidate;
use crate::error::CleanError;
use crate::model::{Candidate, Category, Safety, ScanReport, format_bytes};

use super::{search, theme};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[derive(Clone)]
struct CandidateRow {
    path: String,
    label: String,
    category: Category,
    bytes: u64,
    safety: Safety,
    risk_score: f32,
    reason: String,
    rule_id: String,
}

pub fn run(report: &ScanReport) -> Result<Vec<SelectedCandidate>, CleanError> {
    let mut stdout = io::stdout();
    enable_raw_mode().map_err(clean_error)?;
    execute!(stdout, EnterAlternateScreen).map_err(clean_error)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend).map_err(clean_error)?;
    let mut app = SelectorApp::new(report);

    loop {
        terminal
            .draw(|frame| app.render(frame))
            .map_err(clean_error)?;
        if app.done {
            return Ok(app.selected_candidates());
        }
        if app.cancelled {
            return Ok(Vec::new());
        }
        if event::poll(Duration::from_millis(200)).map_err(clean_error)?
            && let Event::Key(key) = event::read().map_err(clean_error)?
        {
            app.handle_key(key);
        }
    }
}

struct SelectorApp {
    roots: String,
    rows: Vec<CandidateRow>,
    filtered: Vec<usize>,
    selected: BTreeSet<usize>,
    cursor: usize,
    query: String,
    search_mode: bool,
    done: bool,
    cancelled: bool,
}

impl SelectorApp {
    fn new(report: &ScanReport) -> Self {
        let rows = rows_from_report(report);
        let filtered = (0..rows.len()).collect();
        Self {
            roots: report.roots.join(", "),
            rows,
            filtered,
            selected: BTreeSet::new(),
            cursor: 0,
            query: String::new(),
            search_mode: false,
            done: false,
            cancelled: false,
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(frame.area());

        frame.render_widget(
            Paragraph::new(self.header()).block(block("rclean tui")),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(self.controls()).block(block("controls")),
            chunks[1],
        );

        let items = self
            .filtered
            .iter()
            .map(|idx| self.list_item(*idx))
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        if !self.filtered.is_empty() {
            state.select(Some(self.cursor.min(self.filtered.len() - 1)));
        }
        let list = List::new(items)
            .block(block("candidates"))
            .highlight_symbol("> ")
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, chunks[2], &mut state);

        frame.render_widget(
            Paragraph::new(self.explain()).block(block("explain")),
            chunks[3],
        );
    }

    fn header(&self) -> String {
        format!(
            "Roots: {}  Reclaimable: {}  Selected: {} ({})",
            self.roots,
            format_bytes(self.reclaimable_bytes()),
            self.selected.len(),
            format_bytes(self.selected_bytes())
        )
    }

    fn controls(&self) -> String {
        if self.search_mode {
            format!("/{}  enter/esc to leave search", self.query)
        } else {
            "[/]search  [space]toggle  [a]all-safe  [enter]plan  [q]quit".to_string()
        }
    }

    fn list_item(&self, index: usize) -> ListItem<'_> {
        let row = &self.rows[index];
        let selected = self.selected.contains(&index);
        let glyph = glyph(row.safety, selected);
        let text = format!(
            "{} {:<8} {:>10} {:<24} {}",
            glyph,
            row.category,
            format_bytes(row.bytes),
            truncate(&row.label, 24),
            row.path
        );
        ListItem::new(Line::from(vec![Span::styled(
            text,
            theme::candidate_style(row.safety, row.risk_score, selected),
        )]))
    }

    fn explain(&self) -> String {
        let Some(index) = self.filtered.get(self.cursor) else {
            return "no candidates match the current filter".to_string();
        };
        let row = &self.rows[*index];
        format!("{} - {}", row.rule_id, row.reason)
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.search_mode {
            self.handle_search_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cancelled = true;
            }
            KeyCode::Char('q') | KeyCode::Esc => self.cancelled = true,
            KeyCode::Char('/') => self.search_mode = true,
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('a') => self.select_all_safe(),
            KeyCode::Enter => self.done = true,
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.search_mode = false,
            KeyCode::Backspace => {
                self.query.pop();
                self.apply_filter();
            }
            KeyCode::Char(ch) => {
                self.query.push(ch);
                self.apply_filter();
            }
            _ => {}
        }
    }

    fn apply_filter(&mut self) {
        self.filtered = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| search::matches_query(&row.search_text(), &self.query))
            .map(|(index, _)| index)
            .collect();
        self.cursor = self.cursor.min(self.filtered.len().saturating_sub(1));
    }

    fn move_cursor(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let max = self.filtered.len() - 1;
        self.cursor = self.cursor.saturating_add_signed(delta).min(max);
    }

    fn toggle_current(&mut self) {
        let Some(index) = self.filtered.get(self.cursor).copied() else {
            return;
        };
        if self.rows[index].safety == Safety::Blocked {
            return;
        }
        if !self.selected.insert(index) {
            self.selected.remove(&index);
        }
    }

    fn select_all_safe(&mut self) {
        for (index, row) in self.rows.iter().enumerate() {
            if row.safety == Safety::Safe {
                self.selected.insert(index);
            }
        }
    }

    fn selected_candidates(&self) -> Vec<SelectedCandidate> {
        self.selected
            .iter()
            .map(|index| {
                let row = &self.rows[*index];
                SelectedCandidate {
                    id: None,
                    path: PathBuf::from(&row.path),
                    bytes: row.bytes,
                    rule_id: row.rule_id.clone(),
                    category: row.category,
                    safety: row.safety,
                    risk_score: row.risk_score,
                }
            })
            .collect()
    }

    fn selected_bytes(&self) -> u64 {
        self.selected
            .iter()
            .map(|index| self.rows[*index].bytes)
            .sum()
    }

    fn reclaimable_bytes(&self) -> u64 {
        self.rows
            .iter()
            .filter(|row| row.safety != Safety::Blocked)
            .map(|row| row.bytes)
            .sum()
    }
}

impl CandidateRow {
    fn search_text(&self) -> String {
        format!(
            "{} {} {} {}",
            self.path, self.label, self.rule_id, self.category
        )
    }
}

fn rows_from_report(report: &ScanReport) -> Vec<CandidateRow> {
    report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter().map(row_from_candidate))
        .collect()
}

fn row_from_candidate(candidate: &Candidate) -> CandidateRow {
    CandidateRow {
        path: candidate.path.clone(),
        label: candidate.name.clone(),
        category: candidate.category,
        bytes: candidate.bytes,
        safety: candidate.safety,
        risk_score: candidate.risk_score,
        reason: candidate
            .reasons
            .first()
            .or_else(|| candidate.warnings.first())
            .cloned()
            .unwrap_or_else(|| "-".to_string()),
        rule_id: candidate.rule_id.clone(),
    }
}

fn glyph(safety: Safety, selected: bool) -> &'static str {
    if safety == Safety::Blocked {
        "[×]"
    } else if selected {
        "[x]"
    } else if safety == Safety::Caution {
        "[!]"
    } else {
        "[ ]"
    }
}

fn block(title: &'static str) -> Block<'static> {
    Block::default().title(title).borders(Borders::ALL)
}

fn truncate(value: &str, width: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return value.to_string();
    }
    chars
        .into_iter()
        .take(width.saturating_sub(1))
        .chain(std::iter::once('~'))
        .collect()
}

fn clean_error(error: impl std::fmt::Display) -> CleanError {
    CleanError::Generic(format!("tui error: {error}"))
}
