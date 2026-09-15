//! Interactive terminal UI for `tix`, native-only (not part of the npm
//! package or the wasm build). Board view with keyboard navigation, a
//! ticket detail view, a live KEY:VALUE filter, and two mutating actions
//! (`m` move status, `e` set fields). Runs the same `tix-io` commands the
//! CLI does, through [`tix_io::host::CaptureHost`], so what it shows and
//! writes always matches `tix ls`/`tix show`/`tix board`/`tix mv`/`tix set`.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use serde_json::Value;
use std::time::Duration;
use tix_cli::host::{cwd, StdHost};
use tix_io::host::CaptureHost;

/// One row shown in a board column.
struct Row {
    id: String,
    title: String,
    valid: bool,
}

struct Column {
    name: String,
    rows: Vec<Row>,
    selected: usize,
}

enum View {
    Board,
    Detail { ticket: Value, back_col: usize },
    Error(String),
}

/// A modal prompt drawn on top of the current view.
enum Overlay {
    None,
    /// Pick a destination status for `id` (`tix mv`).
    Move {
        id: String,
        statuses: Vec<String>,
        selected: usize,
    },
    /// Free-text `KEY=VALUE ...` line for `id` (`tix set`).
    SetInput { id: String, text: String },
}

struct App {
    root: String,
    host: StdHost,
    columns: Vec<Column>,
    col: usize,
    view: View,
    group: bool,
    status: String,
    overlay: Overlay,
    /// Applied `KEY:VALUE` filter tokens, reused by `tix ls`/`tix board`.
    filter_tokens: Vec<String>,
    /// `Some(text)` while the filter line is being edited.
    filter_input: Option<String>,
    /// `filter_tokens` as they were before the current edit, for `Esc`.
    filter_snapshot: Vec<String>,
}

/// Runs `argv` in-process and returns parsed JSON on success, or the
/// command's stderr. Caller must include `--json --no-prompt` (see
/// [`finish`]).
fn run_json(host: &mut StdHost, root: &str, argv: &[String]) -> Result<Value, String> {
    let mut cap = CaptureHost::new(host);
    let code = tix_io::run(&mut cap, argv, root);
    if code == 0 {
        serde_json::from_str(&cap.out).map_err(|e| format!("bad JSON from tix: {e}"))
    } else {
        Err(cap.err.trim_end().to_string())
    }
}

fn finish(mut argv: Vec<String>) -> Vec<String> {
    argv.push("--json".into());
    argv.push("--no-prompt".into());
    argv
}

fn args(parts: &[&str]) -> Vec<String> {
    finish(parts.iter().map(|s| s.to_string()).collect())
}

impl App {
    fn new(root: String, host: StdHost) -> Self {
        let mut app = App {
            root,
            host,
            columns: Vec::new(),
            col: 0,
            view: View::Board,
            group: false,
            status: String::new(),
            overlay: Overlay::None,
            filter_tokens: Vec::new(),
            filter_input: None,
            filter_snapshot: Vec::new(),
        };
        app.load_board();
        app
    }

    /// Reloads the board and, on failure, replaces the whole view with the
    /// error (used for `r`efresh and anything other than live filter typing).
    fn load_board(&mut self) {
        self.try_load_board(true);
    }

    fn try_load_board(&mut self, hard_error: bool) {
        let mut argv = vec!["board".to_string()];
        argv.extend(self.filter_tokens.clone());
        if self.group {
            argv.push("--group".to_string());
        }
        match run_json(&mut self.host, &self.root, &finish(argv)) {
            Ok(v) => {
                let cols = v["columns"].as_array().cloned().unwrap_or_default();
                self.columns = cols
                    .into_iter()
                    .map(|c| {
                        let name = c["name"].as_str().unwrap_or("?").to_string();
                        let rows = c["tickets"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|t| Row {
                                id: t["id"].as_str().unwrap_or("").to_string(),
                                title: t["title"].as_str().unwrap_or("").to_string(),
                                valid: t["valid"].as_bool().unwrap_or(true),
                            })
                            .collect();
                        Column {
                            name,
                            rows,
                            selected: 0,
                        }
                    })
                    .collect();
                self.col = self.col.min(self.columns.len().saturating_sub(1));
                self.status = format!("{} column(s)", self.columns.len());
            }
            Err(msg) => {
                if hard_error {
                    self.view = View::Error(msg);
                } else {
                    self.status = msg;
                }
            }
        }
    }

    fn current(&self) -> Option<&Row> {
        self.columns
            .get(self.col)
            .and_then(|c| c.rows.get(c.selected))
    }

    /// Id of the ticket currently in view, board selection or detail.
    fn selected_id(&self) -> Option<String> {
        match &self.view {
            View::Board => self.current().map(|r| r.id.clone()),
            View::Detail { ticket, .. } => ticket["id"].as_str().map(|s| s.to_string()),
            View::Error(_) => None,
        }
    }

    fn move_row(&mut self, delta: i32) {
        if let Some(c) = self.columns.get_mut(self.col) {
            if c.rows.is_empty() {
                return;
            }
            let len = c.rows.len() as i32;
            let next = (c.selected as i32 + delta).rem_euclid(len);
            c.selected = next as usize;
        }
    }

    fn move_col(&mut self, delta: i32) {
        if self.columns.is_empty() {
            return;
        }
        let len = self.columns.len() as i32;
        let next = (self.col as i32 + delta).rem_euclid(len);
        self.col = next as usize;
    }

    fn open_detail(&mut self) {
        let Some(row) = self.current() else { return };
        let id = row.id.clone();
        match run_json(&mut self.host, &self.root, &args(&["show", &id])) {
            Ok(ticket) => {
                self.view = View::Detail {
                    ticket,
                    back_col: self.col,
                }
            }
            Err(msg) => self.status = format!("show {id} failed: {msg}"),
        }
    }

    // --- filter ---------------------------------------------------------

    fn start_filter(&mut self) {
        self.filter_snapshot = self.filter_tokens.clone();
        self.filter_input = Some(self.filter_tokens.join(" "));
    }

    fn filter_push(&mut self, c: char) {
        if let Some(text) = &mut self.filter_input {
            text.push(c);
        }
        self.apply_filter_live();
    }

    fn filter_backspace(&mut self) {
        if let Some(text) = &mut self.filter_input {
            text.pop();
        }
        self.apply_filter_live();
    }

    fn apply_filter_live(&mut self) {
        if let Some(text) = &self.filter_input {
            self.filter_tokens = text.split_whitespace().map(|s| s.to_string()).collect();
        }
        self.try_load_board(false);
    }

    fn confirm_filter(&mut self) {
        self.filter_input = None;
        self.load_board();
    }

    fn cancel_filter(&mut self) {
        self.filter_tokens = std::mem::take(&mut self.filter_snapshot);
        self.filter_input = None;
        self.load_board();
    }

    // --- mutations --------------------------------------------------------

    /// All status names, ignoring the current group toggle and filter, so
    /// the move picker always lists every real status.
    fn all_statuses(&mut self) -> Vec<String> {
        match run_json(&mut self.host, &self.root, &args(&["board"])) {
            Ok(v) => v["columns"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|c| c["name"].as_str().map(|s| s.to_string()))
                .filter(|n| n != "?")
                .collect(),
            Err(msg) => {
                self.status = format!("could not load statuses: {msg}");
                Vec::new()
            }
        }
    }

    fn open_move(&mut self) {
        let Some(id) = self.selected_id() else {
            return;
        };
        let statuses = self.all_statuses();
        if statuses.is_empty() {
            return;
        }
        self.overlay = Overlay::Move {
            id,
            statuses,
            selected: 0,
        };
    }

    fn confirm_move(&mut self) {
        let Overlay::Move {
            id,
            statuses,
            selected,
        } = &self.overlay
        else {
            return;
        };
        let id = id.clone();
        let status = statuses[*selected].clone();
        self.overlay = Overlay::None;
        match run_json(&mut self.host, &self.root, &args(&["mv", &id, &status])) {
            Ok(_) => {
                self.status = format!("{id} -> {status}");
                self.after_mutation(&id);
            }
            Err(msg) => self.status = format!("mv failed: {msg}"),
        }
    }

    fn open_set(&mut self) {
        let Some(id) = self.selected_id() else {
            return;
        };
        self.overlay = Overlay::SetInput {
            id,
            text: String::new(),
        };
    }

    fn confirm_set(&mut self) {
        let Overlay::SetInput { id, text } = &self.overlay else {
            return;
        };
        let id = id.clone();
        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
        self.overlay = Overlay::None;
        if tokens.is_empty() {
            return;
        }
        let mut argv = vec!["set".to_string(), id.clone()];
        argv.extend(tokens);
        match run_json(&mut self.host, &self.root, &finish(argv)) {
            Ok(_) => {
                self.status = format!("set {id}");
                self.after_mutation(&id);
            }
            Err(msg) => self.status = format!("set failed: {msg}"),
        }
    }

    /// Reloads the board after a write, and the detail view too if it was
    /// showing the ticket that just changed.
    fn after_mutation(&mut self, id: &str) {
        let refresh_detail = matches!(&self.view, View::Detail { ticket, .. } if ticket["id"].as_str() == Some(id));
        let back_col = match &self.view {
            View::Detail { back_col, .. } => Some(*back_col),
            _ => None,
        };
        self.load_board();
        if refresh_detail {
            if let Some(back_col) = back_col {
                match run_json(&mut self.host, &self.root, &args(&["show", id])) {
                    Ok(ticket) => self.view = View::Detail { ticket, back_col },
                    Err(msg) => self.status = format!("refresh failed: {msg}"),
                }
            }
        }
    }

    // --- render -------------------------------------------------------

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
            .split(area);

        let mut title = vec![
            Span::styled(" tix ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(self.root.clone()),
        ];
        if !self.filter_tokens.is_empty() {
            title.push(Span::raw("  "));
            title.push(Span::styled(
                format!("[{}]", self.filter_tokens.join(" ")),
                Style::default().fg(Color::Yellow),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(title)), chunks[0]);

        match &self.view {
            View::Board => self.draw_board(frame, chunks[1]),
            View::Detail { ticket, .. } => draw_detail(frame, chunks[1], ticket),
            View::Error(msg) => frame.render_widget(
                Paragraph::new(msg.as_str())
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: false }),
                chunks[1],
            ),
        }

        let help: &str = if self.filter_input.is_some() {
            "type KEY:VALUE ...  enter apply  esc cancel"
        } else {
            match &self.overlay {
                Overlay::Move { .. } => "j/k select  enter confirm  esc cancel",
                Overlay::SetInput { .. } => "type KEY=VALUE ...  enter apply  esc cancel",
                Overlay::None => match self.view {
                    View::Board => {
                        "h/l col  j/k row  enter detail  / filter  m move  e set  g group  r refresh  q quit"
                    }
                    View::Detail { .. } => "esc/backspace back  m move  e set  q quit",
                    View::Error(_) => "r retry  q quit",
                },
            }
        };
        let status_text = match &self.filter_input {
            Some(text) => format!("filter: {text}_"),
            None => self.status.clone(),
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(help),
                Span::raw("   "),
                Span::styled(status_text, Style::default().fg(Color::DarkGray)),
            ])),
            chunks[2],
        );

        match &self.overlay {
            Overlay::None => {}
            Overlay::Move {
                id,
                statuses,
                selected,
            } => draw_move_popup(frame, area, id, statuses, *selected),
            Overlay::SetInput { id, text } => draw_set_popup(frame, area, id, text),
        }
    }

    fn draw_board(&self, frame: &mut Frame, area: Rect) {
        if self.columns.is_empty() {
            frame.render_widget(Paragraph::new("no tickets"), area);
            return;
        }
        let pct = 100 / self.columns.len() as u16;
        let constraints: Vec<Constraint> = (0..self.columns.len())
            .map(|i| {
                if i + 1 == self.columns.len() {
                    Constraint::Min(pct)
                } else {
                    Constraint::Percentage(pct)
                }
            })
            .collect();
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        for (i, (col, rect)) in self.columns.iter().zip(cols.iter()).enumerate() {
            let items: Vec<ListItem> = col
                .rows
                .iter()
                .map(|r| {
                    let mark = if r.valid { " " } else { "!" };
                    ListItem::new(format!("{mark}{}", r.title))
                })
                .collect();
            let title = format!("{} ({})", col.name, col.rows.len());
            let block = Block::default().borders(Borders::ALL).title(title);
            let block = if i == self.col {
                block.border_style(Style::default().fg(Color::Cyan))
            } else {
                block
            };
            let list = List::new(items)
                .block(block)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED));
            let mut state = ListState::default();
            if i == self.col && !col.rows.is_empty() {
                state.select(Some(col.selected));
            }
            frame.render_stateful_widget(list, *rect, &mut state);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn draw_move_popup(frame: &mut Frame, area: Rect, id: &str, statuses: &[String], selected: usize) {
    let popup = centered_rect(40, 40, area);
    frame.render_widget(Clear, popup);
    let items: Vec<ListItem> = statuses.iter().map(|s| ListItem::new(s.clone())).collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("move {id} to")),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED));
    let mut state = ListState::default();
    if !statuses.is_empty() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(list, popup, &mut state);
}

fn draw_set_popup(frame: &mut Frame, area: Rect, id: &str, text: &str) {
    let popup = centered_rect(60, 20, area);
    frame.render_widget(Clear, popup);
    let para = Paragraph::new(format!("{text}_")).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("set {id}  (KEY=VALUE, space-separated)")),
    );
    frame.render_widget(para, popup);
}

fn draw_detail(frame: &mut Frame, area: Rect, ticket: &Value) {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("id:     ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(ticket["id"].as_str().unwrap_or("").to_string()),
        ]),
        Line::from(vec![
            Span::styled("title:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(ticket["title"].as_str().unwrap_or("").to_string()),
        ]),
        Line::from(vec![
            Span::styled("status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(ticket["status"].as_str().unwrap_or("").to_string()),
        ]),
    ];
    if let Some(obj) = ticket.as_object() {
        let skip = [
            "id",
            "title",
            "status",
            "created",
            "updated",
            "deliverables",
            "body",
            "valid",
            "problem",
        ];
        for (k, v) in obj {
            if skip.contains(&k.as_str()) {
                continue;
            }
            let text = match v {
                Value::String(s) => s.clone(),
                Value::Array(xs) => xs
                    .iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                other => other.to_string(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{k}: "), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(text),
            ]));
        }
    }
    if let Some(problem) = ticket["problem"].as_str() {
        lines.push(Line::from(Span::styled(
            format!("problem: {problem}"),
            Style::default().fg(Color::Red),
        )));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "brief:",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for l in ticket["body"].as_str().unwrap_or("").lines() {
        lines.push(Line::raw(l.to_string()));
    }
    let block = Block::default().borders(Borders::ALL).title("ticket");
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn run(terminal: &mut DefaultTerminal, mut app: App) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| app.draw(f))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let ctrl_c = key.modifiers.contains(event::KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('c');
        if ctrl_c {
            return Ok(());
        }

        if app.filter_input.is_some() {
            match key.code {
                KeyCode::Enter => app.confirm_filter(),
                KeyCode::Esc => app.cancel_filter(),
                KeyCode::Backspace => app.filter_backspace(),
                KeyCode::Char(c) => app.filter_push(c),
                _ => {}
            }
            continue;
        }

        if matches!(app.overlay, Overlay::SetInput { .. }) {
            match key.code {
                KeyCode::Enter => app.confirm_set(),
                KeyCode::Esc => app.overlay = Overlay::None,
                KeyCode::Backspace => {
                    if let Overlay::SetInput { text, .. } = &mut app.overlay {
                        text.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Overlay::SetInput { text, .. } = &mut app.overlay {
                        text.push(c);
                    }
                }
                _ => {}
            }
            continue;
        }

        if matches!(app.overlay, Overlay::Move { .. }) {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Overlay::Move {
                        statuses, selected, ..
                    } = &mut app.overlay
                    {
                        if !statuses.is_empty() {
                            *selected = (*selected + statuses.len() - 1) % statuses.len();
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Overlay::Move {
                        statuses, selected, ..
                    } = &mut app.overlay
                    {
                        if !statuses.is_empty() {
                            *selected = (*selected + 1) % statuses.len();
                        }
                    }
                }
                KeyCode::Enter => app.confirm_move(),
                KeyCode::Esc => app.overlay = Overlay::None,
                _ => {}
            }
            continue;
        }

        if key.code == KeyCode::Char('q') {
            return Ok(());
        }

        match (&app.view, key.code) {
            (View::Board, KeyCode::Left | KeyCode::Char('h')) => app.move_col(-1),
            (View::Board, KeyCode::Right | KeyCode::Char('l')) => app.move_col(1),
            (View::Board, KeyCode::Up | KeyCode::Char('k')) => app.move_row(-1),
            (View::Board, KeyCode::Down | KeyCode::Char('j')) => app.move_row(1),
            (View::Board, KeyCode::Enter) => app.open_detail(),
            (View::Board, KeyCode::Char('g')) => {
                app.group = !app.group;
                app.load_board();
            }
            (View::Board, KeyCode::Char('r')) => app.load_board(),
            (View::Board, KeyCode::Char('/')) => app.start_filter(),
            (View::Board, KeyCode::Char('m')) => app.open_move(),
            (View::Board, KeyCode::Char('e')) => app.open_set(),
            (View::Error(_), KeyCode::Char('r')) => {
                app.view = View::Board;
                app.load_board();
            }
            (View::Detail { back_col, .. }, KeyCode::Esc | KeyCode::Backspace) => {
                app.col = *back_col;
                app.view = View::Board;
            }
            (View::Detail { .. }, KeyCode::Char('m')) => app.open_move(),
            (View::Detail { .. }, KeyCode::Char('e')) => app.open_set(),
            _ => {}
        }
    }
}

fn main() -> std::io::Result<()> {
    let root = cwd();
    let app = App::new(root, StdHost);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, app);
    ratatui::restore();
    result
}
