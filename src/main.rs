#![allow(dead_code, unused_imports, unused_variables, clippy::all)]

mod app;
mod crypto;
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

    let _ = nix::sys::mman::mlockall(
        nix::sys::mman::MlockAllFlags::MCL_CURRENT
            | nix::sys::mman::MlockAllFlags::MCL_FUTURE,
    );

    app::run()
}
