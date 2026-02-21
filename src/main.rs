#![allow(dead_code, unused_variables, unused_imports, clippy::all)]

mod app;
mod crypto;
mod model;
mod network;
mod panic_engine;
mod protocol;
mod storage;
mod ui;

use std::path::PathBuf;

const GUI_PORT: u16 = 17778;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("null_chat=info")),
        )
        .init();

    // Ensure Python deps are installed quietly
    let _ = std::process::Command::new("pip3")
        .args(["install", "--quiet", "--upgrade", "customtkinter", "pillow"])
        .status();

    // Find gui/main.py relative to working directory / binary
    let gui_path = find_gui_path();

    // Spawn Python GUI in background
    let mut python = std::process::Command::new("python3")
        .arg(&gui_path)
        .arg(GUI_PORT.to_string())
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("[null-chat] Failed to spawn Python GUI: {}", e);
            eprintln!("[null-chat] Make sure python3 is installed.");
            std::process::exit(1);
        });

    // Run the Rust backend (blocks until the GUI disconnects / process exits)
    app::run(GUI_PORT).await;

    let _ = python.kill();
}

fn find_gui_path() -> PathBuf {
    let candidates = [
        PathBuf::from("gui/main.py"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("gui/main.py")))
            .unwrap_or_default(),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    PathBuf::from("gui/main.py")
}

