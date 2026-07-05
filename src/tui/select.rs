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
use crate::model::{Candidate, Category, ProjectReport, Safety, ScanReport, format_bytes};

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
    requires_sudo: bool,
    risk_score: f32,
    reason: String,
    rule_id: String,
    reasons: Vec<String>,
    warnings: Vec<String>,
    restore_hint: String,
    project_kind: String,
    project_markers: Vec<String>,
    git_dirty: Option<bool>,
    last_modified: String,
    staleness_days: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortMode {
    SizeDesc,
    StalenessDesc,
    RiskAsc,
}

impl SortMode {
    fn next(self) -> Self {
        match self {
            Self::SizeDesc => Self::StalenessDesc,
            Self::StalenessDesc => Self::RiskAsc,
            Self::RiskAsc => Self::SizeDesc,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::SizeDesc => "size desc",
            Self::StalenessDesc => "stale desc",
            Self::RiskAsc => "risk asc",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CategoryFilter {
    All,
    Deps,
    Build,
    Cache,
    Test,
}

impl CategoryFilter {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Deps,
            Self::Deps => Self::Build,
            Self::Build => Self::Cache,
            Self::Cache => Self::Test,
            Self::Test => Self::All,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Deps => "deps",
            Self::Build => "build",
            Self::Cache => "cache",
            Self::Test => "test",
        }
    }

    fn matches(self, category: Category) -> bool {
        match self {
            Self::All => true,
            Self::Deps => category == Category::Deps,
            Self::Build => category == Category::Build,
            Self::Cache => category == Category::Cache,
            Self::Test => category == Category::Test,
        }
    }
}

pub fn run(report: &ScanReport) -> Result<Vec<SelectedCandidate>, CleanError> {
    run_with_preselected(report, &BTreeSet::new())
}

pub fn run_with_preselected(
    report: &ScanReport,
    preselected_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<SelectedCandidate>, CleanError> {
    let mut stdout = io::stdout();
    enable_raw_mode().map_err(clean_error)?;
    execute!(stdout, EnterAlternateScreen).map_err(clean_error)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend).map_err(clean_error)?;
    let mut app = SelectorApp::new_with_preselected(report, preselected_paths);

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
    selected: BTreeSet<PathBuf>,
    cursor: usize,
    query: String,
    sort_mode: SortMode,
    category_filter: CategoryFilter,
    search_mode: bool,
    explain_open: bool,
    done: bool,
    cancelled: bool,
}

impl SelectorApp {
    #[cfg(test)]
    fn new(report: &ScanReport) -> Self {
        Self::new_with_preselected(report, &BTreeSet::new())
    }

    fn new_with_preselected(report: &ScanReport, preselected_paths: &BTreeSet<PathBuf>) -> Self {
        let rows = rows_from_report(report);
        let selected = rows
            .iter()
            .filter(|row| row.safety == Safety::Safe && !row.requires_sudo)
            .map(CandidateRow::identity)
            .filter(|path| preselected_paths.contains(path))
            .collect();
        let mut app = Self {
            roots: report.roots.join(", "),
            rows,
            filtered: Vec::new(),
            selected,
            cursor: 0,
            query: String::new(),
            sort_mode: SortMode::SizeDesc,
            category_filter: CategoryFilter::All,
            search_mode: false,
            explain_open: false,
            done: false,
            cancelled: false,
        };
        app.apply_filter();
        app
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

        if self.explain_open {
            frame.render_widget(
                Paragraph::new(self.explain_detail())
                    .block(block("explain (esc or ? to close)"))
                    .wrap(ratatui::widgets::Wrap { trim: false }),
                chunks[2],
            );
            frame.render_widget(
                Paragraph::new(self.explain()).block(block("explain")),
                chunks[3],
            );
            return;
        }

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
            "Roots: {}  Sort: {}  Filter: {}  Reclaimable: {}  Selected: {} ({})",
            self.roots,
            self.sort_mode.label(),
            self.category_filter.label(),
            format_bytes(self.reclaimable_bytes()),
            self.selected.len(),
            format_bytes(self.selected_bytes())
        )
    }

    fn controls(&self) -> String {
        if self.search_mode {
            format!("/{}  enter/esc to leave search", self.query)
        } else {
            "[/]search  [s]sort  [c]category  [space]toggle  [a]all-safe  [?]explain  [enter]plan  [q]quit".to_string()
        }
    }

    fn list_item(&self, index: usize) -> ListItem<'_> {
        let row = &self.rows[index];
        let selected = self.selected.contains(&row.identity());
        let glyph = glyph(row.safety, selected);
        let text = format!(
            "{} {:<8} {:>10} {:>6} {:<24} {}",
            glyph,
            row.category,
            format_bytes(row.bytes),
            crate::output::format_staleness(row.staleness_days),
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

    /// Full-pane rendering of the same content `rclean explain <path>`
    /// prints: rule, safety reasoning, project markers, size, activity.
    fn explain_detail(&self) -> String {
        let Some(index) = self.filtered.get(self.cursor) else {
            return "no candidates match the current filter".to_string();
        };
        let row = &self.rows[*index];
        let mut lines = vec![
            format!("path:      {}", row.path),
            format!("rule:      {}", row.rule_id),
            format!(
                "safety:    {:?}{}",
                row.safety,
                if row.requires_sudo {
                    " (requires sudo; rclean will not delete this)"
                } else {
                    ""
                }
            ),
            format!("size:      {}", format_bytes(row.bytes)),
            format!("risk:      {:.2}", row.risk_score),
            format!("project:   {}", row.project_kind),
            format!(
                "markers:   {}",
                if row.project_markers.is_empty() {
                    "-".to_string()
                } else {
                    row.project_markers.join(", ")
                }
            ),
            format!(
                "git:       {}",
                match row.git_dirty {
                    Some(true) => "dirty worktree",
                    Some(false) => "clean worktree",
                    None => "not a git repository",
                }
            ),
            format!(
                "activity:  last modified {} ({})",
                row.last_modified,
                match row.staleness_days {
                    Some(days) => format!("{days}d ago"),
                    None => "age unknown".to_string(),
                }
            ),
        ];
        if !row.reasons.is_empty() {
            lines.push(String::new());
            lines.push("why this is a candidate:".to_string());
            for reason in &row.reasons {
                lines.push(format!("  - {reason}"));
            }
        }
        if !row.warnings.is_empty() {
            lines.push(String::new());
            lines.push("warnings:".to_string());
            for warning in &row.warnings {
                lines.push(format!("  - {warning}"));
            }
        }
        if !row.restore_hint.is_empty() {
            lines.push(String::new());
            lines.push(format!("restore:   {}", row.restore_hint));
        }
        lines.join("\n")
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
            KeyCode::Char('?') => self.explain_open = !self.explain_open,
            KeyCode::Esc if self.explain_open => self.explain_open = false,
            KeyCode::Char('q') | KeyCode::Esc => self.cancelled = true,
            KeyCode::Char('/') => self.search_mode = true,
            KeyCode::Char('s') => self.cycle_sort(),
            KeyCode::Char('c') => self.cycle_category_filter(),
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
            .filter(|(_, row)| self.category_filter.matches(row.category))
            .filter(|(_, row)| search::matches_query(&row.search_text(), &self.query))
            .map(|(index, _)| index)
            .collect();
        let rows = &self.rows;
        let sort_mode = self.sort_mode;
        self.filtered
            .sort_by(|left, right| compare_rows(&rows[*left], &rows[*right], sort_mode));
        self.cursor = self.cursor.min(self.filtered.len().saturating_sub(1));
    }

    fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.apply_filter();
    }

    fn cycle_category_filter(&mut self) {
        self.category_filter = self.category_filter.next();
        self.apply_filter();
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
        if self.rows[index].safety == Safety::Blocked
            || self.rows[index].safety == Safety::ReportOnly
            || self.rows[index].requires_sudo
        {
            return;
        }
        let identity = self.rows[index].identity();
        if !self.selected.insert(identity.clone()) {
            self.selected.remove(&identity);
        }
    }

    fn select_all_safe(&mut self) {
        for row in &self.rows {
            if row.safety == Safety::Safe && !row.requires_sudo {
                self.selected.insert(row.identity());
            }
        }
    }

    fn selected_candidates(&self) -> Vec<SelectedCandidate> {
        self.rows
            .iter()
            .filter(|row| self.selected.contains(&row.identity()))
            .map(|row| SelectedCandidate {
                id: None,
                path: PathBuf::from(&row.path),
                bytes: row.bytes,
                rule_id: row.rule_id.clone(),
                category: row.category,
                safety: row.safety,
                requires_sudo: row.requires_sudo,
                risk_score: row.risk_score,
            })
            .collect()
    }

    fn selected_bytes(&self) -> u64 {
        self.rows
            .iter()
            .filter(|row| self.selected.contains(&row.identity()))
            .map(|row| row.bytes)
            .sum()
    }

    fn reclaimable_bytes(&self) -> u64 {
        self.rows
            .iter()
            .filter(|row| {
                row.safety != Safety::Blocked
                    && row.safety != Safety::ReportOnly
                    && !row.requires_sudo
            })
            .map(|row| row.bytes)
            .sum()
    }
}

impl CandidateRow {
    fn identity(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    fn search_text(&self) -> String {
        format!(
            "{} {} {} {}",
            self.path, self.label, self.rule_id, self.category
        )
    }
}

fn compare_rows(
    left: &CandidateRow,
    right: &CandidateRow,
    sort_mode: SortMode,
) -> std::cmp::Ordering {
    match sort_mode {
        SortMode::SizeDesc => right.bytes.cmp(&left.bytes),
        SortMode::StalenessDesc => right
            .staleness_days
            .unwrap_or(0)
            .cmp(&left.staleness_days.unwrap_or(0)),
        SortMode::RiskAsc => left.risk_score.total_cmp(&right.risk_score),
    }
    .then(left.path.cmp(&right.path))
}

fn rows_from_report(report: &ScanReport) -> Vec<CandidateRow> {
    report
        .projects
        .iter()
        .flat_map(|project| {
            project
                .candidates
                .iter()
                .map(|candidate| row_from_candidate(project, candidate))
        })
        .collect()
}

fn row_from_candidate(project: &ProjectReport, candidate: &Candidate) -> CandidateRow {
    CandidateRow {
        path: candidate.path.clone(),
        label: candidate.name.clone(),
        category: candidate.category,
        bytes: candidate.bytes,
        safety: candidate.safety,
        requires_sudo: candidate.requires_sudo,
        risk_score: candidate.risk_score,
        reason: candidate
            .reasons
            .first()
            .or_else(|| candidate.warnings.first())
            .cloned()
            .unwrap_or_else(|| "-".to_string()),
        rule_id: candidate.rule_id.clone(),
        reasons: candidate.reasons.clone(),
        warnings: candidate.warnings.clone(),
        restore_hint: candidate.restore_hint.clone(),
        project_kind: project.kind.clone(),
        project_markers: project.markers.clone(),
        git_dirty: project.git.as_ref().map(|git| git.dirty),
        last_modified: project.activity.last_modified.clone(),
        staleness_days: candidate.staleness_days,
    }
}

fn glyph(safety: Safety, selected: bool) -> &'static str {
    if safety == Safety::Blocked {
        "[×]"
    } else if safety == Safety::ReportOnly {
        "[#]"
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

#[cfg(test)]
mod tests;
