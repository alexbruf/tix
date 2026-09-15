//! Interactive terminal UI for `tix`, native-only (not part of the npm
//! package or the wasm build). Read-only first pass: board view with
//! keyboard navigation and a ticket detail view. Runs the same `tix-io`
//! commands the CLI does, through [`tix_io::host::CaptureHost`], so what it
//! shows always matches `tix ls`/`tix show`/`tix board` output.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
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

struct App {
    root: String,
    host: StdHost,
    columns: Vec<Column>,
    col: usize,
    view: View,
    group: bool,
    status: String,
}

/// Runs `argv` in-process (`--json --no-prompt` already applied by the
/// caller) and returns parsed JSON on success, or the command's stderr.
fn run_json(host: &mut StdHost, root: &str, argv: &[String]) -> Result<Value, String> {
    let mut cap = CaptureHost::new(host);
    let code = tix_io::run(&mut cap, argv, root);
    if code == 0 {
        serde_json::from_str(&cap.out).map_err(|e| format!("bad JSON from tix: {e}"))
    } else {
        Err(cap.err.trim_end().to_string())
    }
}

fn args(parts: &[&str]) -> Vec<String> {
    parts
        .iter()
        .map(|s| s.to_string())
        .chain(["--json".to_string(), "--no-prompt".to_string()])
        .collect()
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
        };
        app.load_board();
        app
    }

    fn load_board(&mut self) {
        let argv = if self.group {
            args(&["board", "--group"])
        } else {
            args(&["board"])
        };
        match run_json(&mut self.host, &self.root, &argv) {
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
                self.status = format!("{} column(s) — press ? for help", self.columns.len());
            }
            Err(msg) => self.view = View::Error(msg),
        }
    }

    fn current(&self) -> Option<&Row> {
        self.columns
            .get(self.col)
            .and_then(|c| c.rows.get(c.selected))
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

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" tix ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(self.root.clone()),
            ])),
            chunks[0],
        );

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

        let help = match self.view {
            View::Board => "h/l or ←/→ column  j/k or ↑/↓ row  enter detail  g group  r refresh  q quit",
            View::Detail { .. } => "esc/backspace back  q quit",
            View::Error(_) => "r retry  q quit",
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(help),
                Span::raw("   "),
                Span::styled(self.status.clone(), Style::default().fg(Color::DarkGray)),
            ])),
            chunks[2],
        );
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
            let list = List::new(items).block(block).highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            );
            let mut state = ListState::default();
            if i == self.col && !col.rows.is_empty() {
                state.select(Some(col.selected));
            }
            frame.render_stateful_widget(list, *rect, &mut state);
        }
    }
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
        if key.code == KeyCode::Char('q') || ctrl_c {
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
            (View::Error(_), KeyCode::Char('r')) => {
                app.view = View::Board;
                app.load_board();
            }
            (View::Detail { back_col, .. }, KeyCode::Esc | KeyCode::Backspace) => {
                app.col = *back_col;
                app.view = View::Board;
            }
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
