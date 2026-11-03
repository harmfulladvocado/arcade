use std::{io, time::{Duration, Instant}};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, poll};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

const W: usize = 10;
const H: usize = 20;
const TICK: Duration = Duration::from_millis(500);

type Grid = [[Option<Color>; W]; H];

// each piece: list of (row, col) offsets from origin
const PIECES: &[(&[&[(i8, i8)]], Color)] = &[
    // I
    (&[
        &[(0,0),(0,1),(0,2),(0,3)],
        &[(0,0),(1,0),(2,0),(3,0)],
    ], Color::Cyan),
    // O
    (&[
        &[(0,0),(0,1),(1,0),(1,1)],
    ], Color::Yellow),
    // T
    (&[
        &[(0,0),(0,1),(0,2),(1,1)],
        &[(0,0),(1,0),(2,0),(1,1)],    // rotated — note: simplified rotations
        &[(1,0),(1,1),(1,2),(0,1)],
        &[(0,1),(1,1),(2,1),(1,0)],
    ], Color::Magenta),
    // S
    (&[
        &[(0,1),(0,2),(1,0),(1,1)],
        &[(0,0),(1,0),(1,1),(2,1)],
    ], Color::Green),
    // Z
    (&[
        &[(0,0),(0,1),(1,1),(1,2)],
        &[(0,1),(1,0),(1,1),(2,0)],
    ], Color::Red),
    // J
    (&[
        &[(0,0),(1,0),(1,1),(1,2)],
        &[(0,0),(0,1),(1,0),(2,0)],
        &[(0,0),(0,1),(0,2),(1,2)],
        &[(0,1),(1,1),(2,0),(2,1)],
    ], Color::Blue),
    // L
    (&[
        &[(0,2),(1,0),(1,1),(1,2)],
        &[(0,0),(1,0),(2,0),(2,1)],
        &[(0,0),(0,1),(0,2),(1,0)],
        &[(0,0),(0,1),(1,1),(2,1)],
    ], Color::LightRed),
];

struct Piece {
    kind: usize,
    rot: usize,
    row: i8,
    col: i8,
}

impl Piece {
    fn cells(&self) -> &[(i8, i8)] {
        let (rotations, _) = PIECES[self.kind];
        rotations[self.rot % rotations.len()]
    }

    fn color(&self) -> Color {
        PIECES[self.kind].1
    }

    fn absolute_cells(&self) -> Vec<(i8, i8)> {
        self.cells().iter().map(|(dr, dc)| (self.row + dr, self.col + dc)).collect()
    }
}

struct State {
    grid: Grid,
    piece: Piece,
    next_kind: usize,
    score: u32,
    level: u32,
    lines_cleared: u32,
    alive: bool,
    last_tick: Instant,
}

fn random_piece_kind() -> usize {
    use rand::Rng;
    rand::thread_rng().gen_range(0..PIECES.len())
}

fn new_piece(kind: usize) -> Piece {
    Piece { kind, rot: 0, row: 0, col: (W as i8 / 2) - 1 }
}

impl State {
    fn new() -> Self {
        let mut s = Self {
            grid: [[None; W]; H],
            piece: new_piece(random_piece_kind()),
            next_kind: random_piece_kind(),
            score: 0,
            level: 1,
            lines_cleared: 0,
            alive: true,
            last_tick: Instant::now(),
        };
        if !s.valid(&s.piece.absolute_cells()) { s.alive = false; }
        s
    }

    fn valid(&self, cells: &[(i8, i8)]) -> bool {
        cells.iter().all(|&(r, c)| {
            r >= 0 && c >= 0 && r < H as i8 && c < W as i8
                && self.grid[r as usize][c as usize].is_none()
        })
    }

    fn lock(&mut self) {
        let color = self.piece.color();
        for (r, c) in self.piece.absolute_cells() {
            if r >= 0 && c >= 0 {
                self.grid[r as usize][c as usize] = Some(color);
            }
        }
        self.clear_lines();

        self.piece = new_piece(self.next_kind);
        self.next_kind = random_piece_kind();

        if !self.valid(&self.piece.absolute_cells()) {
            self.alive = false;
        }
    }

    fn clear_lines(&mut self) {
        let mut cleared = 0u32;
        let mut new_grid: Grid = [[None; W]; H];
        let mut write_row = H - 1;

        for r in (0..H).rev() {
            if self.grid[r].iter().all(|c| c.is_some()) {
                cleared += 1;
            } else {
                new_grid[write_row] = self.grid[r];
                if write_row > 0 { write_row -= 1; }
            }
        }

        self.grid = new_grid;
        self.lines_cleared += cleared;
        self.score += match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        } * self.level;
        self.level = self.lines_cleared / 10 + 1;
    }

    fn try_move(&mut self, dr: i8, dc: i8) -> bool {
        let cells: Vec<(i8, i8)> = self.piece.absolute_cells()
            .iter().map(|&(r, c)| (r + dr, c + dc)).collect();
        if self.valid(&cells) {
            self.piece.row += dr;
            self.piece.col += dc;
            true
        } else {
            false
        }
    }

    fn try_rotate(&mut self) {
        let (rotations, _) = PIECES[self.piece.kind];
        let new_rot = (self.piece.rot + 1) % rotations.len();
        let cells: Vec<(i8, i8)> = rotations[new_rot].iter()
            .map(|&(dr, dc)| (self.piece.row + dr, self.piece.col + dc))
            .collect();

        // try wall kicks: center, left, right
        for dc in [0i8, 1, -1, 2, -2] {
            let kicked: Vec<(i8, i8)> = cells.iter().map(|&(r, c)| (r, c + dc)).collect();
            if self.valid(&kicked) {
                self.piece.rot = new_rot;
                self.piece.col += dc;
                return;
            }
        }
    }

    fn hard_drop(&mut self) {
        while self.try_move(1, 0) {}
        self.lock();
    }

    fn tick(&mut self) {
        let speed = TICK.checked_sub(Duration::from_millis((self.level.saturating_sub(1) * 40) as u64))
            .unwrap_or(Duration::from_millis(80));

        if self.last_tick.elapsed() >= speed {
            if !self.try_move(1, 0) {
                self.lock();
            }
            self.last_tick = Instant::now();
        }
    }

    fn ghost_row(&self) -> i8 {
        let mut dr = 0i8;
        loop {
            let cells: Vec<(i8, i8)> = self.piece.absolute_cells()
                .iter().map(|&(r, c)| (r + dr + 1, c)).collect();
            if self.valid(&cells) { dr += 1; } else { break; }
        }
        self.piece.row + dr
    }
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut state = State::new();

    loop {
        if poll(Duration::from_millis(0))? {
            if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                if kind == KeyEventKind::Press {
                    match code {
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        KeyCode::Left | KeyCode::Char('a') => { state.try_move(0, -1); }
                        KeyCode::Right | KeyCode::Char('d') => { state.try_move(0, 1); }
                        KeyCode::Down | KeyCode::Char('s') => { state.try_move(1, 0); }
                        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('r') => state.try_rotate(),
                        KeyCode::Char(' ') => state.hard_drop(),
                        _ => {}
                    }
                }
            }
        }

        if state.alive { state.tick(); }

        terminal.draw(|frame| {
            let area = frame.size();
            // game board + sidebar
            let total_w = (W * 2 + 2 + 14) as u16;
            let total_h = (H + 2) as u16;
            let game_area = center_rect(total_w, total_h, area);

            let board_w = (W * 2 + 2) as u16;
            let board_area = Rect::new(game_area.x, game_area.y, board_w, total_h);
            let side_area = Rect::new(game_area.x + board_w, game_area.y, 14, total_h);

            let ghost_row = state.ghost_row();
            let piece_cells: Vec<(i8, i8)> = state.piece.absolute_cells();

            let lines: Vec<Line> = (0..H).map(|r| {
                let mut spans = Vec::new();
                for c in 0..W {
                    // is this cell part of the active piece?
                    let is_piece = piece_cells.iter().any(|&(pr, pc)| pr as usize == r && pc as usize == c);
                    // ghost
                    let is_ghost = !is_piece && piece_cells.iter().any(|&(pr, pc)| {
                        (ghost_row + (pr - state.piece.row)) as usize == r && pc as usize == c
                    });

                    let (ch, style) = if is_piece {
                        ("[]", Style::default().fg(state.piece.color()))
                    } else if is_ghost {
                        ("[]", Style::default().fg(Color::DarkGray))
                    } else if let Some(col) = state.grid[r][c] {
                        ("[]", Style::default().fg(col))
                    } else {
                        (". ", Style::default().fg(Color::DarkGray))
                    };
                    spans.push(Span::styled(ch, style));
                }
                Line::from(spans)
            }).collect();

            let game_over_line = if !state.alive {
                Line::from(Span::styled(" GAME OVER - press any key ", Style::default().fg(Color::Red)))
            } else {
                Line::from(format!(" Score: {}  Level: {}  Lines: {} ", state.score, state.level, state.lines_cleared))
            };

            let mut board_lines = lines;
            board_lines.push(game_over_line);

            let board = Paragraph::new(board_lines)
                .block(Block::default().borders(Borders::ALL).title(" Tetris "));
            frame.render_widget(board, board_area);

            // sidebar: next piece + controls
            let (next_rots, next_color) = PIECES[state.next_kind];
            let next_cells = next_rots[0];
            let mut preview = vec![vec![false; 4]; 4];
            for &(r, c) in next_cells {
                if r >= 0 && c >= 0 && r < 4 && c < 4 {
                    preview[r as usize][c as usize] = true;
                }
            }

            let mut side_lines: Vec<Line> = vec![Line::from("Next:"), Line::from("")];
            for row in &preview {
                let spans: Vec<Span> = row.iter().map(|&filled| {
                    if filled {
                        Span::styled("[]", Style::default().fg(next_color))
                    } else {
                        Span::raw("  ")
                    }
                }).collect();
                side_lines.push(Line::from(spans));
            }

            side_lines.push(Line::from(""));
            side_lines.push(Line::from(format!("Lv {}", state.level)));
            side_lines.push(Line::from(""));
            side_lines.push(Line::from("AD:move"));
            side_lines.push(Line::from("S:down"));
            side_lines.push(Line::from("W/R:rot"));
            side_lines.push(Line::from("Spc:drop"));
            side_lines.push(Line::from("Q:quit"));

            let sidebar = Paragraph::new(side_lines)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(sidebar, side_area);
        })?;

        if !state.alive {
            wait_for_key()?;
            break;
        }

        std::thread::sleep(Duration::from_millis(16));
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