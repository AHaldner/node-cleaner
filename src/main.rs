use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};
use std::io;

#[derive(Debug, Clone)]
pub struct NodeModuleItem {
    pub path: String,
    pub size: String,
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

impl Default for AppData {
    fn default() -> Self {
        Self {
            items: vec![
                NodeModuleItem {
                    path: "./projects/web-app/node_modules".to_string(),
                    size: "450 MB".to_string(),
                    last_modified: "2024-01-15".to_string(),
                },
                NodeModuleItem {
                    path: "./projects/api-server/node_modules".to_string(),
                    size: "120 MB".to_string(),
                    last_modified: "2024-02-20".to_string(),
                },
                NodeModuleItem {
                    path: "./experiments/react-test/node_modules".to_string(),
                    size: "850 MB".to_string(),
                    last_modified: "2023-11-05".to_string(),
                },
                NodeModuleItem {
                    path: "./work/client-portal/node_modules".to_string(),
                    size: "1.2 GB".to_string(),
                    last_modified: "2024-03-01".to_string(),
                },
                NodeModuleItem {
                    path: "./legacy/old-site/node_modules".to_string(),
                    size: "300 MB".to_string(),
                    last_modified: "2022-08-12".to_string(),
                },
            ],
        }
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
                Row::new(vec![
                    Cell::from(item.path.clone()),
                    Cell::from(item.size.clone()),
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

impl Default for App {
    fn default() -> Self {
        Self {
            data: AppData::default(),
            state: AppState::new(),
            exit: false,
        }
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
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}
