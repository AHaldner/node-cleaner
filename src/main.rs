use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use files::{find_node_modules, format_modified, format_size};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};
use std::{io, path::Path};

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
}

impl AppData {
    pub fn from_path(root: &Path) -> io::Result<Self> {
        let entries = find_node_modules(root)?;
        let items = entries
            .into_iter()
            .map(|entry| NodeModuleItem {
                path: entry.path.display().to_string(),
                size: format_size(entry.size_bytes),
                size_bytes: entry.size_bytes,
                last_modified: format_modified(entry.modified),
            })
            .collect();
        Ok(Self { items })
    }
}

impl StatefulWidget for &AppData {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let title = Line::from(" Node Cleaner ".bold().magenta());
        let instructions = Line::from(vec![
            " Navigate ".into(),
            "<Up/Down>".magenta().bold(),
            " Quit ".into(),
            "<Q> ".magenta().bold(),
        ]);

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

                Row::new(vec![
                    Cell::from(item.path.clone()),
                    Cell::from(item.size.clone().fg(size_color)),
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

        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

#[derive(Debug)]
pub struct App {
    data: AppData,
    state: AppState,
    exit: bool,
}

impl App {
    pub fn new(root: &Path) -> io::Result<Self> {
        Ok(Self {
            data: AppData::from_path(root)?,
            state: AppState::new(),
            exit: false,
        })
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_stateful_widget(&self.data, frame.area(), &mut self.state);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up => self.state.previous(self.data.items.len()),
            KeyCode::Down => self.state.next(self.data.items.len()),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

fn main() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let mut terminal = ratatui::init();
    let app_result = App::new(&root)?.run(&mut terminal);
    ratatui::restore();
    app_result
}
