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
