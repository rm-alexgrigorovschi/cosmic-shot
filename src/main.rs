use anyhow::Context;
use tracing_subscriber::EnvFilter;

// These items are used via lib.rs; suppress dead_code for the binary target.
#[allow(dead_code)]
mod capture;
mod config;
mod overlay;
mod types;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
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
