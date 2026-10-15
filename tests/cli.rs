use std::process::Command;

fn view() -> Command {
    Command::new(env!("CARGO_BIN_EXE_view"))
}

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixture.txt");

#[test]
fn quiet_no_pager_prints_only_content() {
    let out = view().args(["--no-pager", "-q", FIXTURE]).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("um texto"));
    assert!(!text.contains('$'));
    assert!(!text.contains('─'));
}

#[test]
fn header_shows_command_and_ruler() {
    let out = view().args(["--no-pager", FIXTURE]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("\x1b[2m$ "));
    assert!(text.contains('─'));
    assert!(text.contains("um texto"));
}

#[test]
fn dry_run_prints_command_without_running() {
    let out = view().args(["-n", FIXTURE]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("$ "));
    assert!(text.trim_end().ends_with("fixture.txt"));
    assert!(!text.contains("um texto"));
}

#[test]
fn unknown_format_fails() {
    let out = view().args(["--as", "nope", FIXTURE]).output().unwrap();
    assert!(!out.status.success());
}
