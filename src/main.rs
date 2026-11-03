mod games;
mod music;

use std::io;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use games::{galaga, snake, tetris};

const TITLE: &str = concat!(
"    _    ____   ____    _    ____  _____\n",
"   / \\  |  _ \\ / ___|  / \\  |  _ \\| ____|\n",
" / _ \\ | |_) | |     / _ \\ | | | |  _|\n",
"/ ___ \\|  _ <| |___ / ___ \\| |_| | |___\n",
"/_/   \\_\\_| \\_\\\\____/_/   \\_\\____/|_____|",
);

const MENU_ITEMS: &[(&str, &str)] = &[
    ("1", "Snake"),
    ("2", "Galaga"),
    ("3", "Tetris"),
    ("Q", "Quit"),
];

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let _audio = music::start();

    let result = run_menu(&mut terminal);

    execute!(io::stdout(), terminal::LeaveAlternateScreen, crossterm::cursor::Show)?;
    terminal::disable_raw_mode()?;

    result
}

fn run_menu(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            let area = frame.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7),
                    Constraint::Length(2),
                    Constraint::Min(0),
                ])
                .split(area);

            let title = Paragraph::new(TITLE)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(title, chunks[0]);

            let subtitle = Paragraph::new("Select a game")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            frame.render_widget(subtitle, chunks[1]);

            let items: Vec<Line> = MENU_ITEMS.iter().map(|(key, label)| {
                Line::from(vec![
                    Span::styled(format!("  [{}]  ", key), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(label.to_string()),
                ])
            }).collect();

            let menu = Paragraph::new(items)
                .block(Block::default().borders(Borders::NONE));
            frame.render_widget(menu, chunks[2]);
        })?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Char('1') => snake::run(terminal)?,
                KeyCode::Char('2') => galaga::run(terminal)?,
                KeyCode::Char('3') => tetris::run(terminal)?,
                KeyCode::Char('q') | KeyCode::Char('Q') => break,
                _ => {}
            }
        }
    }

    Ok(())
}