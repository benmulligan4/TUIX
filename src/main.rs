/// TUIX — Terminal UI Experience (Rust/ratatui port)
/// Entry point — mirrors main.py

mod tuix;
mod settings;
mod touchscreen;
mod dashboards;
mod applications;
mod utilities;
mod app_store;

fn main() {
    tuix::app::main();
}
