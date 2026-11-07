# Arcade

A terminal arcade written in Rust. It runs a menu with three playable games (Snake, Galaga, Tetris) and plays a looping chiptune melody in the background. The interface is rendered with `ratatui` and driven by keyboard input from `crossterm`.

## Table of contents

- [Features](#features)
- [Entry point](#entry-point)
- [Game modules](#game-modules)
- [Audio](#audio)
- [Module map](#module-map)
- [Installation](#installation)
  - [Prerequisites](#prerequisites)
  - [Build from source](#build-from-source)
  - [Run](#run)
- [Controls](#controls)
- [License](#license)

## Features

- Full screen terminal UI with an ASCII title and menu.
- Three games: Snake, Galaga, and Tetris.
- Looping chiptune melody generated at runtime.
- Clean terminal restore after exit.

### Entry point

`src/main.rs` starts the program. It enables raw mode, enters the alternate screen, hides the cursor, and creates the `ratatui` terminal. It then starts the audio thread through `music::start` and enters the menu loop.

The menu draws the title, a subtitle, and a list of options. Each key press dispatches to the matching game function, and quitting resets the terminal to its original state.

```text
main
  -> enable raw mode, alternate screen
  -> start music
  -> run_menu
       -> draw title and options
       -> read key
       -> snake::run / galaga::run / tetris::run
  -> restore terminal
```

### Game modules

Each game lives in `src/games/` and exposes a single `run` function with the same signature. A game owns its own state, timing, input handling, and rendering loop, then returns control to the menu.

- `snake.rs`: grid based snake with a `VecDeque` body, random food, and fixed tick timing.
- `galaga.rs`: vertical shooter with player movement, bullets, enemies, and explosion effects.
- `tetris.rs`: falling block puzzle with piece rotation, a color grid, and line clearing.

`src/games/mod.rs` declares the three modules so the rest of the crate can import them.

### Audio

`src/music.rs` defines a `MelodySource` that implements both `Iterator<f32>` and `rodio::Source`. It generates a sine wave for each note in a fixed melody array, using a zero frequency for rests. `start` opens an output stream and returns the stream and sink so the menu can keep them alive. If no audio device is available, it returns `None` and the game continues silently.

### Module map

| Path                | Responsibility                              |
|:--------------------|:--------------------------------------------|
| `src/main.rs`       | Terminal setup, menu loop, game dispatch    |
| `src/music.rs`      | Background melody and audio output          |
| `src/games/mod.rs`  | Game module declarations                    |
| `src/games/snake.rs`| Snake game logic and rendering              |
| `src/games/galaga.rs`| Galaga game logic and rendering            |
| `src/games/tetris.rs`| Tetris game logic and rendering            |

## Installation

### Requirements

- Rust toolchain with `cargo`.
- A terminal that supports alternate screen and raw mode.
- An audio output device for the music (optional, the game runs without it).
- On Linux, the ALSA development libraries are required by `rodio`.

```bash
# Debian based
sudo apt install libasound2-dev

# Fedora based
sudo dnf install alsa-lib-devel

# Arch based
sudo pacman -S alsa-lib
```

### Build from source

```bash
git clone https://github.com/harmfulladvocado/arcade
cd Arcade
cargo build --release
```

### Run

```bash
cargo run --release
```

## Controls

- Menu: press `1` for Snake, `2` for Galaga, `3` for Tetris, `Q` to quit.
- Each game shows its own controls on screen and returns to the menu with the exit key.

## License

This project is licensed under the MIT License. See [LICENSE](./LICENSE) for details.
