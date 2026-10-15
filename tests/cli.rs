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

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/examples");

/// Roda `view -n` sobre um exemplo e devolve o resultado do processo.
fn dry_run(example: &str) -> std::process::Output {
    view()
        .args(["-n", &format!("{EXAMPLES}/{example}")])
        .output()
        .unwrap()
}

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn dry_run_elf_without_extension_is_hexdump() {
    let out = dry_run("program");
    let text = stdout_of(&out);
    assert!(out.status.success());
    assert!(text.contains("hexyl") || text.contains("xxd"), "{text}");
    assert!(!text.contains("bat"), "{text}");
}

#[test]
fn dry_run_binary_bin_is_hexdump() {
    let text = stdout_of(&dry_run("binary.bin"));
    assert!(text.contains("hexyl") || text.contains("xxd"), "{text}");
}

#[test]
fn dry_run_png_renamed_to_txt_is_not_text() {
    // Imagem ainda não tem backend: o `view` falha em vez de cair no `bat`.
    let out = dry_run("image-as.txt");
    let text = stdout_of(&out);
    assert!(!text.contains("bat"), "{text}");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("nenhum visualizador disponível para image"),
        "{err}"
    );
}

#[test]
fn dry_run_extensionless_text_uses_bat() {
    let text = stdout_of(&dry_run("LICENSE"));
    assert!(text.contains("bat"), "{text}");
}

#[test]
fn dry_run_text_file_uses_bat() {
    let text = stdout_of(&dry_run("notes.txt"));
    assert!(text.contains("bat"), "{text}");
}

#[test]
fn dry_run_directory_lists_it() {
    let out = dry_run("dir");
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(text.contains("dir"), "{text}");
    assert!(!text.contains("bat"), "{text}");
}

#[test]
fn as_flag_overrides_magic_bytes() {
    let out = view()
        .args(["-n", "--as", "text", &format!("{EXAMPLES}/program")])
        .output()
        .unwrap();
    assert!(stdout_of(&out).contains("bat"));
}
