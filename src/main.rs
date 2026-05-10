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
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--print-shortcut") {
        let cfg = config::Config::load();
        println!("Shortcut: {}", cfg.shortcut);
        println!("Command:  cosmic-shot");
        return Ok(());
    }

    // Parse --delay N
    let cli_delay: Option<u64> = parse_delay_flag(&args)?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("starting cosmic-shot");

    let cfg = config::Config::load();

    // Resolve delay: CLI flag takes precedence over config.
    let delay_secs = cli_delay.unwrap_or(cfg.delay_secs);

    if delay_secs > 0 {
        run_countdown(delay_secs).await;
    }

    let frames = capture::capture_all_outputs()
        .context("failed to capture outputs")?;

    tracing::info!("captured {} output(s)", frames.len());

    overlay::run(frames).context("overlay error")?;

    Ok(())
}

/// Parse `--delay N` from the argument list.
///
/// Returns `Ok(Some(n))` if `--delay N` is present and valid,
/// `Ok(None)` if `--delay` is absent,
/// `Err` if `--delay` is present but the value is missing or not a u64.
fn parse_delay_flag(args: &[String]) -> anyhow::Result<Option<u64>> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--delay" {
            let val = iter
                .next()
                .ok_or_else(|| anyhow::anyhow!("--delay requires a value (e.g. --delay 3)"))?;
            let secs: u64 = val.parse().map_err(|_| {
                anyhow::anyhow!("--delay value must be a non-negative integer, got {:?}", val)
            })?;
            let secs = secs.min(60);
            return Ok(Some(secs));
        }
    }
    Ok(None)
}

/// Print a countdown to stdout and sleep until capture time.
async fn run_countdown(delay_secs: u64) {
    for remaining in (1..=delay_secs).rev() {
        println!("Capturing in {}...", remaining);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
