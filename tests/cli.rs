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

/// Roda `view` com `bytes` no stdin (em pipe) e devolve o resultado.
fn view_stdin(args: &[&str], bytes: &[u8]) -> std::process::Output {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = view()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let data = bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();
    out
}

fn example_bytes(name: &str) -> Vec<u8> {
    std::fs::read(format!("{EXAMPLES}/{name}")).unwrap()
}

#[test]
fn stdin_text_is_viewed_with_bat_or_cat() {
    let out = view_stdin(&["--no-pager", "-q"], &example_bytes("notes.txt"));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_of(&out);
    assert!(text.contains("planejamento"), "{text}");
    let dry = stdout_of(&view_stdin(&["-n"], &example_bytes("notes.txt")));
    assert!(
        dry.starts_with("$ bat ") || dry.starts_with("$ cat "),
        "{dry}"
    );
    assert!(dry.trim_end().ends_with(" -"), "{dry}");
}

#[test]
fn stdin_dash_is_the_same_as_no_target() {
    let a = stdout_of(&view_stdin(&["-n"], b"ola\n"));
    let b = stdout_of(&view_stdin(&["-n", "-"], b"ola\n"));
    assert_eq!(a, b);
}

#[test]
fn stdin_markdown_with_as_shows_glow() {
    let out = view_stdin(&["--as", "md", "-n"], b"# a");
    let text = stdout_of(&out);
    assert!(out.status.success());
    assert!(
        text.starts_with("$ glow ") || text.starts_with("$ bat "),
        "{text}"
    );
    assert!(text.trim_end().ends_with(" -"), "{text}");
}

#[test]
fn stdin_markdown_is_rendered() {
    let out = view_stdin(
        &["--as", "md", "--no-pager", "-q"],
        &example_bytes("readme.md"),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.stdout.is_empty());
}

#[test]
fn stdin_json_is_detected_as_text_and_viewed() {
    let json = example_bytes("data.json");
    let out = view_stdin(&["--no-pager", "-q"], &json);
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(text.contains("descricao"), "{text}");
}

#[test]
fn stdin_binary_is_a_hexdump() {
    let bytes = example_bytes("binary.bin");
    let dry = stdout_of(&view_stdin(&["-n"], &bytes));
    assert!(dry.contains("hexyl") || dry.contains("xxd"), "{dry}");
    let out = view_stdin(&["--no-pager", "-q"], &bytes);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout_of(&out).contains("00000000"));
}

#[test]
fn stdin_image_is_detected_by_magic_bytes() {
    let out = view_stdin(&["-n"], &example_bytes("image.png"));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("nenhum visualizador disponível para image"),
        "{err}"
    );
}

#[test]
fn stdin_for_path_only_backend_uses_a_private_temp_file_that_is_removed() {
    let out = view_stdin(&["--as", "dir", "--no-pager"], b"conteudo\n");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout_of(&out);
    let first = text.lines().next().unwrap();
    let path = first
        .rsplit(' ')
        .next()
        .unwrap()
        .trim_end_matches("\x1b[0m");
    assert!(path.contains("view-stdin-"), "{first}");
    assert!(
        !std::path::Path::new(path).exists(),
        "{path} ficou para trás"
    );
}

#[test]
fn stdin_dry_run_with_path_only_backend_shows_temp_path_without_creating_it() {
    let out = view_stdin(&["--as", "dir", "-n"], b"x");
    let text = stdout_of(&out);
    let path = text.trim_end().rsplit(' ').next().unwrap();
    assert!(path.contains("view-stdin-"), "{text}");
    assert!(!std::path::Path::new(path).exists());
}

#[test]
fn stdin_toml_gets_highlighted() {
    let out = view_stdin(&["--no-pager", "-q"], &example_bytes("config.toml"));
    assert!(out.status.success());
    let dry = stdout_of(&view_stdin(&["-n"], &example_bytes("config.toml")));
    if dry.starts_with("$ bat ") {
        assert!(dry.contains("-l toml"), "{dry}");
        assert!(stdout_of(&out).contains("\x1b["));
    }
}
