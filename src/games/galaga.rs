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
use rand::Rng;

const W: u16 = 42;
const H: u16 = 24;

const SHOOT_COOLDOWN: Duration = Duration::from_millis(300);
const PLAYER_TICK: Duration = Duration::from_millis(55);
const BULLET_TICK: Duration = Duration::from_millis(75);
const ENEMY_TICK: Duration = Duration::from_millis(550);
const ENEMY_SHOOT_TICK: Duration = Duration::from_millis(1100);
const EXPLOSION_TTL: Duration = Duration::from_millis(200);

#[derive(Clone, Copy)]
struct Pos { x: u16, y: u16 }

struct State {
    player_x: u16,
    lives: u8,
    invincible_until: Option<Instant>,

    bullets: Vec<Pos>,
    enemy_bullets: Vec<Pos>,
    enemies: Vec<(Pos, bool)>,   // (pos, alive)
    explosions: Vec<(Pos, Instant)>,

    score: u32,
    wave: u32,
    enemy_dir: i16,

    last_player_move: Instant,
    last_bullet_move: Instant,
    last_enemy_move: Instant,
    last_enemy_shoot: Instant,
    last_shot: Instant,

    move_left: bool,
    move_right: bool,
    shooting: bool,
    running: bool,
}

impl State {
    fn new() -> Self {
        Self {
            player_x: W / 2,
            lives: 3,
            invincible_until: None,
            bullets: Vec::new(),
            enemy_bullets: Vec::new(),
            enemies: spawn_wave(1),
            explosions: Vec::new(),
            score: 0,
            wave: 1,
            enemy_dir: 1,
            last_player_move: Instant::now(),
            last_bullet_move: Instant::now(),
            last_enemy_move: Instant::now(),
            last_enemy_shoot: Instant::now(),
            last_shot: Instant::now(),
            move_left: false,
            move_right: false,
            shooting: false,
            running: true,
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();

        if now.duration_since(self.last_player_move) >= PLAYER_TICK {
            if self.move_left && self.player_x > 1 { self.player_x -= 1; }
            if self.move_right && self.player_x < W - 2 { self.player_x += 1; }
            self.last_player_move = now;
        }

        if self.shooting && now.duration_since(self.last_shot) >= SHOOT_COOLDOWN {
            self.bullets.push(Pos { x: self.player_x, y: H - 3 });
            self.last_shot = now;
        }

        if now.duration_since(self.last_bullet_move) >= BULLET_TICK {
            for b in &mut self.bullets { b.y = b.y.saturating_sub(1); }
            self.bullets.retain(|b| b.y > 0);
            for b in &mut self.enemy_bullets { b.y += 1; }
            self.enemy_bullets.retain(|b| b.y < H - 1);
            self.last_bullet_move = now;
        }

        if now.duration_since(self.last_enemy_move) >= ENEMY_TICK {
            let hit_wall = self.enemies.iter().any(|(p, alive)| {
                *alive && ((self.enemy_dir > 0 && p.x >= W - 2) || (self.enemy_dir < 0 && p.x <= 1))
            });

            if hit_wall {
                self.enemy_dir *= -1;
                for (p, alive) in &mut self.enemies {
                    if *alive { p.y += 1; }
                }
            } else {
                for (p, alive) in &mut self.enemies {
                    if *alive { p.x = (p.x as i16 + self.enemy_dir) as u16; }
                }
            }
            self.last_enemy_move = now;
        }

        if now.duration_since(self.last_enemy_shoot) >= ENEMY_SHOOT_TICK {
            let alive: Vec<Pos> = self.enemies.iter()
                .filter(|(_, a)| *a)
                .map(|(p, _)| *p)
                .collect();
            if !alive.is_empty() {
                let idx = rand::thread_rng().gen_range(0..alive.len());
                self.enemy_bullets.push(Pos { x: alive[idx].x, y: alive[idx].y + 1 });
            }
            self.last_enemy_shoot = now;
        }

        self.check_collisions(now);
        self.explosions.retain(|(_, t)| now.duration_since(*t) < EXPLOSION_TTL);

        if self.enemies.iter().all(|(_, a)| !a) {
            self.wave += 1;
            self.enemies = spawn_wave(self.wave);
            self.bullets.clear();
            self.enemy_bullets.clear();
        }

        let reached_bottom = self.enemies.iter().any(|(p, a)| *a && p.y >= H - 3);
        if reached_bottom || self.lives == 0 {
            self.running = false;
        }
    }

    fn check_collisions(&mut self, now: Instant) {
        for bullet in &mut self.bullets {
            for (pos, alive) in &mut self.enemies {
                if *alive && pos.x == bullet.x && pos.y == bullet.y {
                    *alive = false;
                    self.score += 10;
                    self.explosions.push((*pos, now));
                    bullet.y = 0;
                }
            }
        }

        let invincible = self.invincible_until.map(|t| now < t).unwrap_or(false);
        if !invincible {
            for bullet in &mut self.enemy_bullets {
                if bullet.x == self.player_x && bullet.y == H - 3 {
                    self.lives = self.lives.saturating_sub(1);
                    self.invincible_until = Some(now + Duration::from_millis(1500));
                    bullet.y = H;
                    break;
                }
            }
        }
    }
}

fn spawn_wave(wave: u32) -> Vec<(Pos, bool)> {
    let rows = (wave as u16 + 1).min(4);
    let cols = 8u16;
    let start_x = (W - cols * 4) / 2;
    let mut enemies = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            enemies.push((Pos { x: start_x + col * 4, y: 2 + row * 2 }, true));
        }
    }
    enemies
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut state = State::new();

    loop {
        while poll(Duration::from_millis(0))? {
            if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                let pressed = kind == KeyEventKind::Press || kind == KeyEventKind::Repeat;
                match code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                    KeyCode::Left | KeyCode::Char('a') => state.move_left = pressed,
                    KeyCode::Right | KeyCode::Char('d') => state.move_right = pressed,
                    KeyCode::Char(' ') => state.shooting = pressed,
                    _ => {}
                }
            }
        }

        state.tick();

        let now = Instant::now();
        terminal.draw(|frame| {
            let area = frame.size();
            let game_area = center_rect(W + 2, H + 3, area);

            let mut grid: Vec<Vec<(&str, Color)>> = vec![vec![(" ", Color::Reset); W as usize]; H as usize];

            // enemies
            for (pos, alive) in &state.enemies {
                if *alive && pos.x < W && pos.y < H {
                    let is_exploding = state.explosions.iter()
                        .any(|(ep, t)| ep.x == pos.x && ep.y == pos.y && now.duration_since(*t) < EXPLOSION_TTL);
                    let (ch, col) = if is_exploding { ("*", Color::Yellow) } else { ("V", Color::Magenta) };
                    grid[pos.y as usize][pos.x as usize] = (ch, col);
                }
            }

            // explosions on cleared cells
            for (pos, t) in &state.explosions {
                if now.duration_since(*t) < EXPLOSION_TTL && pos.x < W && pos.y < H {
                    grid[pos.y as usize][pos.x as usize] = ("*", Color::Yellow);
                }
            }

            // player bullets
            for b in &state.bullets {
                if b.x < W && b.y < H { grid[b.y as usize][b.x as usize] = ("|", Color::Cyan); }
            }

            // enemy bullets
            for b in &state.enemy_bullets {
                if b.x < W && b.y < H { grid[b.y as usize][b.x as usize] = ("v", Color::Red); }
            }

            // player
            let invincible = state.invincible_until.map(|t| now < t).unwrap_or(false);
            let py = (H - 3) as usize;
            let px = state.player_x as usize;
            if px < W as usize && py < H as usize {
                let col = if invincible { Color::DarkGray } else { Color::Green };
                grid[py][px] = ("^", col);
            }

            let lines: Vec<Line> = grid.iter().map(|row| {
                Line::from(row.iter().map(|(ch, col)| {
                    Span::styled(ch.to_string(), Style::default().fg(*col))
                }).collect::<Vec<_>>())
            }).collect();

            let lives_str: String = (0..state.lives).map(|_| "^ ").collect();
            let hud = if !state.running {
                Line::from(Span::styled(" GAME OVER - press any key ", Style::default().fg(Color::Red)))
            } else {
                Line::from(format!(
                    " Score: {}  Wave: {}  Lives: {}  [A/D] move  [Space] shoot  [Q] quit ",
                    state.score, state.wave, lives_str.trim()
                ))
            };

            let mut all_lines = lines;
            all_lines.push(hud);

            let widget = Paragraph::new(all_lines)
                .block(Block::default().borders(Borders::ALL).title(" Space Shooter "));
            frame.render_widget(widget, game_area);
        })?;

        if !state.running {
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