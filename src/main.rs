use crate::delete::{DeleteEvent, spawn_delete};
use crate::files::ScanEvent;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use files::{NodeModulesEntry, format_modified, format_size, scan_node_modules};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{
        Block, Cell, Clear, Gauge, HighlightSpacing, Paragraph, Row, StatefulWidget, Table,
        TableState, Widget,
    },
};
use std::{
    io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

mod delete;
mod files;

#[derive(Debug, Clone)]
pub struct NodeModuleItem {
    pub path: String,
    pub size: String,
    pub size_bytes: u64,
    pub last_modified: String,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub table_state: TableState,
}

impl AppState {
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self { table_state: state }
    }

    pub fn next(&mut self, item_count: usize) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= item_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self, item_count: usize) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    item_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
}

#[derive(Debug)]
pub struct AppData {
    pub items: Vec<NodeModuleItem>,
    pub total_discovered: u64,
    pub removed_count: u64,
    pub status_message: Option<String>,
    pub currently_deleting: Option<String>,
}

impl AppData {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_discovered: 0,
            removed_count: 0,
            status_message: None,
            currently_deleting: None,
        }
    }

    pub fn add_entry(&mut self, entry: NodeModulesEntry) {
        self.items.push(NodeModuleItem {
            path: entry.path.display().to_string(),
            size: format_size(entry.size_bytes),
            size_bytes: entry.size_bytes,
            last_modified: format_modified(entry.modified),
        });
    }

    pub fn increment_discovered(&mut self) {
        self.total_discovered += 1;
    }

    pub fn visible_total(&self) -> u64 {
        self.total_discovered.saturating_sub(self.removed_count)
    }

    pub fn remove_entry(&mut self, index: usize) -> bool {
        if index < self.items.len() {
            self.items.remove(index);
            self.removed_count += 1;
            true
        } else {
            false
        }
    }

    pub fn remove_by_path(&mut self, path: &str) -> Option<usize> {
        if let Some(index) = self.items.iter().position(|item| item.path == path) {
            self.items.remove(index);
            self.removed_count += 1;
            Some(index)
        } else {
            None
        }
    }

    pub fn set_status_message(&mut self, message: Option<String>) {
        self.status_message = message;
    }

    pub fn set_currently_deleting(&mut self, path: Option<String>) {
        self.currently_deleting = path;
    }
}

impl StatefulWidget for &AppData {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let processed_count = self.items.len() as u64;
        let visible_total = self.visible_total();
        let show_progress = processed_count < visible_total || visible_total == 0;

        let constraints = if show_progress {
            vec![Constraint::Min(0), Constraint::Length(3)]
        } else {
            vec![Constraint::Min(0)]
        };

        let layout = Layout::vertical(constraints);
        let chunks = layout.split(area);
        let table_area = chunks[0];

        let title = Line::from(" Node Cleaner ".bold().magenta());
        let mut instruction_spans = vec![
            " Navigate ".into(),
            "<Up/Down>".magenta().bold(),
            " Remove ".into(),
            "<Enter>".magenta().bold(),
            " Quit ".into(),
            "<Q> ".magenta().bold(),
        ];

        if let Some(status) = &self.status_message {
            instruction_spans.push(" | ".into());
            instruction_spans.push(status.clone().into());
        }

        let instructions = Line::from(instruction_spans);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK)
            .border_style(Style::new().white());

        let widths = [
            Constraint::Min(40),
            Constraint::Length(10),
            Constraint::Length(15),
        ];

        let rows: Vec<Row> = self
            .items
            .iter()
            .map(|item| {
                let size_color = match item.size_bytes {
                    s if s >= 1_073_741_824 => Color::Red,
                    s if s >= 524_288_000 => Color::Yellow,
                    _ => Color::Green,
                };

                let is_deleting = self
                    .currently_deleting
                    .as_ref()
                    .map(|p| p == &item.path)
                    .unwrap_or(false);

                let mut path_cell = item.path.clone();
                if is_deleting {
                    path_cell.push_str(" (deleting...)");
                }

                Row::new(vec![
                    Cell::from(path_cell).style(if is_deleting {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
                    Cell::from(item.size.clone()).fg(size_color),
                    Cell::from(item.last_modified.clone()),
                ])
            })
            .collect();

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["Path", "Size", "Modified"])
                    .style(
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .block(block)
            .highlight_symbol(" → ")
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(table, table_area, buf, &mut state.table_state);

        if show_progress {
            let ratio = if visible_total > 0 {
                processed_count as f64 / visible_total as f64
            } else {
                0.0
            };

            let ratio = ratio.clamp(0.0, 1.0);

            let gauge = Gauge::default()
                .block(Block::bordered().title(" Scanning Progress "))
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(ratio)
                .label(format!("{}/{} scanned", processed_count, visible_total));

            gauge.render(chunks[1], buf);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

#[derive(Debug)]
pub struct App {
    data: AppData,
    state: AppState,
    exit: bool,
    rx: Receiver<ScanEvent>,
    delete_tx: mpsc::Sender<DeleteEvent>,
    delete_rx: Receiver<DeleteEvent>,
    pending_delete: Option<usize>,
    root: PathBuf,
}

impl App {
    pub fn new(root: &Path) -> io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let (delete_tx, delete_rx) = mpsc::channel();
        let root = root.canonicalize()?;
        let scan_root = root.clone();

        thread::spawn(move || {
            scan_node_modules(scan_root, tx);
        });

        Ok(Self {
            data: AppData::new(),
            state: AppState::new(),
            exit: false,
            rx,
            delete_tx,
            delete_rx,
            pending_delete: None,
            root,
        })
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            while let Ok(event) = self.rx.try_recv() {
                match event {
                    ScanEvent::Discovered => self.data.increment_discovered(),
                    ScanEvent::Entry(entry) => self.data.add_entry(entry),
                }
            }

            while let Ok(event) = self.delete_rx.try_recv() {
                self.handle_delete_event(event);
            }

            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_stateful_widget(&self.data, frame.area(), &mut self.state);
        self.draw_confirm_modal(frame);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.pending_delete.is_some() {
            match key_event.code {
                KeyCode::Enter | KeyCode::Char('y') => self.confirm_delete(),
                KeyCode::Esc | KeyCode::Char('n') => self.cancel_delete(),
                _ => {}
            }
            return;
        }

        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Enter => self.prompt_delete(),
            KeyCode::Up if !self.data.items.is_empty() => {
                self.state.previous(self.data.items.len())
            }
            KeyCode::Down if !self.data.items.is_empty() => self.state.next(self.data.items.len()),
            _ => {}
        }
    }

    fn prompt_delete(&mut self) {
        if let Some(ref active) = self.data.currently_deleting {
            self.data
                .set_status_message(Some(format!("Already deleting: {}", active)));
            return;
        }

        if let Some(selected) = self.state.table_state.selected() {
            if selected < self.data.items.len() {
                self.pending_delete = Some(selected);
            }
        }
    }

    fn confirm_delete(&mut self) {
        if let Some(index) = self.pending_delete.take() {
            let Some(path) = self.data.items.get(index).map(|item| item.path.clone()) else {
                self.data
                    .set_status_message(Some("Delete failed: item missing".into()));
                return;
            };

            self.data
                .set_status_message(Some(format!("Deleting: {}", path)));
            self.data.set_currently_deleting(Some(path.clone()));

            spawn_delete(path, self.root.clone(), self.delete_tx.clone());
        }
    }

    fn cancel_delete(&mut self) {
        self.pending_delete = None;
    }

    fn handle_delete_event(&mut self, event: DeleteEvent) {
        match event {
            DeleteEvent::Started(path) => {
                self.data
                    .set_status_message(Some(format!("Deleting: {}", path)));
                self.data.set_currently_deleting(Some(path));
            }
            DeleteEvent::Finished { path, result } => match result {
                Ok(_) => {
                    self.data.set_status_message(None);
                    self.data.set_currently_deleting(None);
                    if let Some(index) = self.data.remove_by_path(&path) {
                        self.adjust_selection_after_remove(index);
                    }
                }
                Err(err) => {
                    self.data
                        .set_status_message(Some(format!("Delete failed: {err}")));
                    self.data.set_currently_deleting(None);
                }
            },
        }
    }

    fn adjust_selection_after_remove(&mut self, removed_index: usize) {
        let len = self.data.items.len();
        if len == 0 {
            self.state.table_state.select(None);
            return;
        }

        let current = self.state.table_state.selected().unwrap_or(0);
        let new_index = if current > removed_index {
            current.saturating_sub(1)
        } else {
            current.min(len.saturating_sub(1))
        };

        self.state.table_state.select(Some(new_index));
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn draw_confirm_modal(&self, frame: &mut Frame) {
        let Some(index) = self.pending_delete else {
            return;
        };

        let Some(item) = self.data.items.get(index) else {
            return;
        };

        let area = centered_rect(60, 30, frame.area());

        let message = vec![
            Line::from("Remove this entry from the list?").bold(),
            Line::from(""),
            Line::from(item.path.clone()),
            Line::from(""),
            Line::from("Enter / y: remove    Esc / n: cancel"),
        ];

        let paragraph = Paragraph::new(message).alignment(Alignment::Center).block(
            Block::bordered()
                .title(" Confirm removal ")
                .border_set(border::THICK),
        );

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }
}

fn main() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let mut terminal = ratatui::init();
    let app_result = App::new(&root)?.run(&mut terminal);
    ratatui::restore();
    app_result
}
