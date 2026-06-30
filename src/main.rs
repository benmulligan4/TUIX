/// TUIX — Terminal UI Experience (Rust/ratatui port)
/// Entry point — mirrors main.py

mod tuix;
mod settings;
mod touchscreen;
mod dashboards;
mod applications;
mod utilities;

fn main() {
    tuix::app::main();
}
