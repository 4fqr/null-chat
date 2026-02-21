#![allow(dead_code, unused_variables, unused_imports, clippy::all)]

mod app;
mod crypto;
mod model;
mod network;
mod panic_engine;
mod protocol;
mod storage;
mod ui;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("null_chat=info")),
        )
        .init();

    app::run()
}
