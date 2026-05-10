use std::process::Command;

#[test]
fn print_shortcut_prints_two_lines_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .arg("--print-shortcut")
        .output()
        .expect("failed to run cosmic-shot");

    assert!(
        output.status.success(),
        "exit code was not 0: {:?}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected exactly 2 lines, got: {:?}", lines);
    assert!(
        lines[0].starts_with("Shortcut:"),
        "first line should start with 'Shortcut:': {:?}",
        lines[0]
    );
    assert!(
        lines[1].starts_with("Command:"),
        "second line should start with 'Command:': {:?}",
        lines[1]
    );
    assert!(
        lines[1].contains("cosmic-shot"),
        "command line should contain 'cosmic-shot': {:?}",
        lines[1]
    );
}

#[test]
fn delay_flag_missing_value_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .arg("--delay")
        .output()
        .expect("failed to run cosmic-shot");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --delay has no value"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("--delay requires a value") || stderr.contains("--delay"),
        "stderr should mention --delay: {:?}",
        stderr
    );
}

#[test]
fn delay_flag_non_integer_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .args(["--delay", "abc"])
        .output()
        .expect("failed to run cosmic-shot");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --delay value is not an integer"
    );
}

#[test]
#[ignore = "requires a non-interactive environment (no Wayland display)"]
fn delay_flag_zero_does_not_hang() {
    use std::time::{Duration, Instant};

    let mut child = Command::new(env!("CARGO_BIN_EXE_cosmic-shot"))
        .args(["--delay", "0"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to spawn cosmic-shot");

    let start = Instant::now();
    let deadline = Duration::from_secs(5);
    let mut exited_quickly = false;

    while start.elapsed() < deadline {
        match child.try_wait().expect("failed to poll process") {
            Some(_) => {
                exited_quickly = true;
                break;
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }

    if !exited_quickly {
        let _ = child.kill();
    }

    assert!(
        exited_quickly,
        "--delay 0 should not run a countdown; process should exit quickly (< 5s)"
    );
}
