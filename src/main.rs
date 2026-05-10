use anyhow::Context;
use tracing_subscriber::EnvFilter;

// These items are used via lib.rs; suppress dead_code for the binary target.
#[allow(dead_code)]
mod capture;
mod config;
mod export;
mod overlay;
mod types;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Handle CLI flags before initialising tracing or touching Wayland.
    if std::env::args().any(|a| a == "--print-shortcut") {
        let cfg = config::Config::load();
        println!("Shortcut: {}", cfg.shortcut);
        println!("Command:  cosmic-shot");
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("starting cosmic-shot");

    let frames = capture::capture_all_outputs()
        .context("failed to capture outputs")?;

    tracing::info!("captured {} output(s)", frames.len());

    overlay::run(frames).context("overlay error")?;

    Ok(())
}
