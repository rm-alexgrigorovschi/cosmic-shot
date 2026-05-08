mod capture;
mod export;
mod overlay;
mod types;

use std::path::Path;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("cosmic-shot starting");

    // Phase 1: Capture
    let frame = capture::capture_output()?;
    tracing::info!(
        width = frame.width,
        height = frame.height,
        "capture complete"
    );

    // Phase 2: Export (verification side effect)
    let output_path = Path::new("capture.png");
    export::save_png(&frame, output_path)?;

    // Phase 3: Display
    overlay::run(frame)?;

    Ok(())
}
