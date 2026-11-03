use std::{collections::VecDeque, io, time::{Duration, Instant}};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, poll};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use rand::Rng;

const W: u16 = 20;
const H: u16 = 20;
const TICK: Duration = Duration::from_millis(120);

#[derive(Clone, Copy, PartialEq)]
struct Pos { x: i16, y: i16 }

#[derive(Clone, Copy, PartialEq)]
enum Dir { Up, Down, Left, Right }

impl Dir {
    fn delta(self) -> (i16, i16) {
        match self {
            Dir::Up    => (0, -1),
            Dir::Down  => (0,  1),
            Dir::Left  => (-1, 0),
            Dir::Right => (1,  0),
        }
    }
    fn opposite(self) -> Dir {
        match self {
            Dir::Up    => Dir::Down,
            Dir::Down  => Dir::Up,
            Dir::Left  => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

struct State {
    snake: VecDeque<Pos>,
    dir: Dir,
    next_dir: Dir,
    food: Pos,
    score: u32,
    alive: bool,
}

impl State {
    fn new() -> Self {
        let head = Pos { x: (W / 2) as i16, y: (H / 2) as i16 };
        let mut snake = VecDeque::new();
        snake.push_back(head);
        snake.push_back(Pos { x: head.x - 1, y: head.y });
        snake.push_back(Pos { x: head.x - 2, y: head.y });
        let food = spawn_food(&snake);
        Self { snake, dir: Dir::Right, next_dir: Dir::Right, food, score: 0, alive: true }
    }

    fn tick(&mut self) {
        self.dir = self.next_dir;
        let (dx, dy) = self.dir.delta();
        let head = *self.snake.front().unwrap();
        let next = Pos { x: head.x + dx, y: head.y + dy };

        if next.x < 0 || next.x >= W as i16 || next.y < 0 || next.y >= H as i16
            || self.snake.contains(&next)
        {
            self.alive = false;
            return;
        }

        self.snake.push_front(next);
        if next == self.food {
            self.score += 10;
            self.food = spawn_food(&self.snake);
        } else {
            self.snake.pop_back();
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        let dir = match key {
            KeyCode::Up    | KeyCode::Char('w') => Some(Dir::Up),
            KeyCode::Down  | KeyCode::Char('s') => Some(Dir::Down),
            KeyCode::Left  | KeyCode::Char('a') => Some(Dir::Left),
            KeyCode::Right | KeyCode::Char('d') => Some(Dir::Right),
            _ => None,
        };
        if let Some(d) = dir {
            if d != self.dir.opposite() { self.next_dir = d; }
        }
    }
}

fn spawn_food(snake: &VecDeque<Pos>) -> Pos {
    let mut rng = rand::thread_rng();
    loop {
        let p = Pos { x: rng.gen_range(0..W as i16), y: rng.gen_range(0..H as i16) };
        if !snake.contains(&p) { return p; }
    }
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut state = State::new();
    let mut last_tick = Instant::now();

    loop {
        if poll(Duration::from_millis(0))? {
            if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                if kind == KeyEventKind::Press {
                    match code {
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        _ => state.handle_key(code),
                    }
                }
            }
        }

        if last_tick.elapsed() >= TICK {
            state.tick();
            last_tick = Instant::now();
        }

        terminal.draw(|frame| {
            let area = frame.size();
            let game_area = center_rect(W * 2 + 2, H + 3, area);

            let mut grid: Vec<Vec<(&str, Color)>> =
                vec![vec![("  ", Color::Reset); W as usize]; H as usize];

            let (fx, fy) = (state.food.x as usize, state.food.y as usize);
            if fx < W as usize && fy < H as usize {
                grid[fy][fx] = ("<>", Color::Red);
            }

            for (i, seg) in state.snake.iter().enumerate() {
                if seg.x >= 0 && seg.x < W as i16 && seg.y >= 0 && seg.y < H as i16 {
                    let color = if i == 0 { Color::White } else { Color::Green };
                    grid[seg.y as usize][seg.x as usize] = ("[]", color);
                }
            }

            let lines: Vec<Line> = grid.iter().map(|row| {
                Line::from(row.iter().map(|(ch, color)| {
                    Span::styled(ch.to_string(), Style::default().fg(*color))
                }).collect::<Vec<_>>())
            }).collect();

            let status = if !state.alive {
                Line::from(Span::styled(
                    " GAME OVER - press any key ",
                    Style::default().fg(Color::Red),
                ))
            } else {
                Line::from(format!(" Score: {}  [WASD/arrows] move  [Q] quit ", state.score))
            };

            let mut all_lines = lines;
            all_lines.push(status);

            let widget = Paragraph::new(all_lines)
                .block(Block::default().borders(Borders::ALL).title(" Snake "));
            frame.render_widget(widget, game_area);
        })?;

        if !state.alive {
            wait_for_key()?;
            break;
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn center_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

fn wait_for_key() -> io::Result<()> {
    loop {
        if let Event::Key(_) = event::read()? { return Ok(()); }
    }
}