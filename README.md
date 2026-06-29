# TUIX (Rust)

TUIX is a terminal UI shell built with `ratatui` in Rust.

It starts with a clean framed interface, a header made of expandable tiles, a default dashboard, and a small app manager that can launch external apps as subprocesses.

## What is included right now

- A framed TUI shell with a top tile bar
- Header tiles for:
  - Dashboard
  - Apps
  - Settings
  - Touchscreen
  - System
- Two dashboards:
  - Dashboard 1 — live clock & date
  - Dashboard 2 — system stats (CPU, memory, disk)
- Placeholder pages for Settings and Touchscreen
- A System page showing running processes
- An internal **Character Set** app that displays printable Unicode character ranges

## Controls

| Key | Action                                |
|-----|---------------------------------------|
| `W` / `↑` | Move up                               |
| `S` / `↓` | Move down                             |
| `A` / `←` | Move left                             |
| `D` / `→` | Move right                            |
| `Enter` / `E` | Select / open                         |
| `Tab` | Toggle focus: Navbar ↔ Main container |
| `Q` / `Backspace` | Go back / collapse dropdown           |
| `Esc` | Quit TUIX                             |

## Building and Running

```bash
cd TUIX_rust
cargo build --release
cargo run
```

## Project Structure

```
TUIX_rust/
├── Cargo.toml
├── config/
│   ├── apps.json
│   ├── dashboards.json
│   └── settings.json
├── library/
│   ├── INIT_TREE_STRUCTURE.txt
│   ├── REQUIREMENTS.md
│   └── context.txt
├── src/
│   ├── main.rs
│   ├── tuix/
│   │   ├── mod.rs
│   │   ├── app.rs
│   │   ├── input_handler.rs
│   │   ├── models.rs
│   │   ├── process_manager.rs
│   │   └── registry.rs
│   ├── settings/
│   │   ├── mod.rs
│   │   └── page.rs
│   ├── touchscreen/
│   │   ├── mod.rs
│   │   └── page.rs
│   ├── dashboards/
│   │   ├── mod.rs
│   │   ├── dashboard_1.rs
│   │   └── dashboard_2.rs
│   └── applications/
│       ├── mod.rs
│       └── character_set.rs
```
