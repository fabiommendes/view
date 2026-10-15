use std::process::Command;

fn view() -> Command {
    Command::new(env!("CARGO_BIN_EXE_view"))
}

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixture.txt");

#[test]
fn default_prints_only_content() {
    let out = view().args(["--no-pager", FIXTURE]).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("um texto"));
    assert!(!text.contains('$'));
    assert!(!text.contains('─'));
}

#[test]
fn verbose_header_shows_command_and_ruler() {
    let out = view().args(["--no-pager", "-v", FIXTURE]).output().unwrap();
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
    let out = dry_run("image-as.txt");
    let text = stdout_of(&out);
    assert!(out.status.success(), "{out:?}");
    assert!(!text.contains("bat"), "{text}");
    assert!(text.contains("imagem"), "{text}");
}

/// Roda `view` sobre um exemplo com o stdout num pipe (sem terminal).
fn view_example(example: &str, args: &[&str]) -> std::process::Output {
    view()
        .args(args)
        .arg(format!("{EXAMPLES}/{example}"))
        .env_remove("KITTY_WINDOW_ID")
        .env("TERM_PROGRAM", "ghostty")
        .output()
        .unwrap()
}

#[test]
fn image_files_become_half_blocks_when_stdout_is_a_pipe() {
    // Mesmo com um terminal kitty anunciado no ambiente: pipe não é terminal.
    for name in ["image.png", "photo.jpg", "image-as.txt"] {
        let out = view_example(name, &[]);
        assert!(out.status.success(), "{name}: {out:?}");
        let text = stdout_of(&out);
        assert!(text.contains("\x1b[38;2;"), "{name}");
        assert!(text.contains('▀'), "{name}");
        assert!(!text.contains("\x1b_G"), "{name}");
    }
}

#[test]
fn image_header_names_the_protocol_before_the_image() {
    let text = stdout_of(&view_example("image.png", &["-v"]));
    let note = text
        .find("# imagem: meio-bloco")
        .expect("nota no cabeçalho");
    assert!(note < text.find('▀').unwrap());
}

#[test]
fn image_ignores_no_pager() {
    let with = view_example("image.png", &["--no-pager"]);
    let without = view_example("image.png", &[]);
    assert_eq!(with.stdout, without.stdout);
}

#[test]
fn image_fits_the_default_width() {
    let text = stdout_of(&view_example("photo.jpg", &[]));
    for line in text.lines() {
        assert!(line.matches('▀').count() <= 80, "{line}");
    }
}

#[test]
fn image_that_cannot_be_decoded_fails() {
    let out = view_example("notes.txt", &["--as", "image"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("decodificar"));
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
    let out = view_stdin(&["--no-pager"], &example_bytes("notes.txt"));
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
    let out = view_stdin(&["--as", "md", "--no-pager"], &example_bytes("readme.md"));
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
    let out = view_stdin(&["--no-pager"], &json);
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(text.contains("descricao"), "{text}");
}

#[test]
fn stdin_binary_is_a_hexdump() {
    let bytes = example_bytes("binary.bin");
    let dry = stdout_of(&view_stdin(&["-n"], &bytes));
    assert!(dry.contains("hexyl") || dry.contains("xxd"), "{dry}");
    let out = view_stdin(&["--no-pager"], &bytes);
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
    assert!(stdout_of(&out).contains("imagem"), "{out:?}");
    let out = view_stdin(&[], &example_bytes("image.png"));
    assert!(out.status.success(), "{out:?}");
    assert!(stdout_of(&out).contains('▀'));
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
    let out = view_stdin(&["--no-pager"], &example_bytes("config.toml"));
    assert!(out.status.success());
    let dry = stdout_of(&view_stdin(&["-n"], &example_bytes("config.toml")));
    if dry.starts_with("$ bat ") {
        assert!(dry.contains("-l toml"), "{dry}");
        assert!(stdout_of(&out).contains("\x1b["));
    }
}

mod structured {
    use super::{EXAMPLES, stdout_of, view};
    use std::io::Write;
    use std::process::{Command, Output, Stdio};

    fn run(format: &str, example: &str) -> Output {
        view()
            .args([
                "--no-pager",
                "--as",
                format,
                &format!("{EXAMPLES}/{example}"),
            ])
            .output()
            .unwrap()
    }

    fn run_stdin(format: &str, content: &[u8]) -> Output {
        run_stdin_args(&["--no-pager", "--as", format], content)
    }

    fn run_stdin_args(args: &[&str], content: &[u8]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_view"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(content).unwrap();
        child.wait_with_output().unwrap()
    }

    /// Remove sequências de cor ANSI.
    fn plain(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn json_of(out: &Output) -> serde_json::Value {
        serde_json::from_str(&plain(&stdout_of(out))).unwrap()
    }

    fn example(name: &str) -> String {
        std::fs::read_to_string(format!("{EXAMPLES}/{name}")).unwrap()
    }

    #[test]
    fn json_to_yaml_and_toml_files() {
        let yaml = run("yaml", "data-no-null.json");
        assert!(yaml.status.success());
        let text = plain(&stdout_of(&yaml));
        assert!(text.contains("projeto:"), "{text}");
        assert!(text.contains("nome: view"), "{text}");
        let toml = run("toml", "data-no-null.json");
        assert!(toml.status.success());
        let text = plain(&stdout_of(&toml));
        assert!(text.contains("[projeto]"), "{text}");
    }

    #[test]
    fn yaml_and_toml_files_to_other_formats() {
        for (format, example_name) in [
            ("json", "config.yaml"),
            ("toml", "config.yaml"),
            ("json", "config.toml"),
            ("yaml", "config.toml"),
        ] {
            let out = run(format, example_name);
            assert!(out.status.success(), "{format} {example_name}");
            let text = plain(&stdout_of(&out));
            assert!(text.contains("servidor"), "{text}");
            assert!(text.contains("8080"), "{text}");
        }
    }

    #[test]
    fn json_yaml_json_round_trip_through_the_cli() {
        let yaml = run("yaml", "data.json");
        assert!(yaml.status.success());
        let back = run_stdin("json", plain(&stdout_of(&yaml)).as_bytes());
        assert!(back.status.success());
        let original: serde_json::Value = serde_json::from_str(&example("data.json")).unwrap();
        let round = json_of(&back);
        assert_eq!(round, original);
        assert_eq!(round.to_string(), original.to_string(), "ordem das chaves");
    }

    #[test]
    fn stdin_converts_too() {
        let out = run_stdin("yaml", example("data-no-null.json").as_bytes());
        assert!(out.status.success());
        assert!(plain(&stdout_of(&out)).contains("projeto:"));
        let out = run_stdin("json", example("config.toml").as_bytes());
        assert!(out.status.success());
        assert_eq!(json_of(&out)["servidor"]["porta"], 8080);
    }

    #[test]
    fn null_to_toml_fails_with_a_clear_error() {
        let out = run("toml", "data.json");
        assert!(!out.status.success());
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.starts_with("view: "), "{err}");
        assert!(err.contains("json para toml"), "{err}");
        assert!(!err.contains("panicked"), "{err}");
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn invalid_source_fails_with_a_clear_error() {
        let out = run_stdin_args(&["--no-pager", "--as", "yaml"], b"{\"a\": ");
        assert!(!out.status.success());
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("view: "), "{err}");
        assert!(!err.contains("panicked"), "{err}");
    }

    #[test]
    fn raw_json_does_not_convert() {
        let raw = example("data.json");
        for args in [vec!["--raw", "--as", "json"], vec!["--raw"]] {
            let out = view()
                .args(["--no-pager"])
                .args(&args)
                .arg(format!("{EXAMPLES}/data.json"))
                .output()
                .unwrap();
            assert!(out.status.success());
            // O `bat` mantém o texto original, inclusive o `null` que o TOML rejeitaria.
            assert_eq!(plain(&stdout_of(&out)), raw, "{args:?}");
        }
    }

    #[test]
    fn header_shows_conversion_and_sink() {
        let out = view()
            .args(["-n", "--as", "yaml", &format!("{EXAMPLES}/data.json")])
            .output()
            .unwrap();
        let text = stdout_of(&out);
        assert!(
            text.starts_with(&format!(
                "$ view --as yaml {EXAMPLES}/data.json  # json→yaml interno | bat "
            )),
            "{text}"
        );
        assert!(text.trim_end().ends_with("-l yaml"), "{text}");
    }
}

mod json {
    use super::{stdout_of, view};
    use std::io::Write;
    use std::process::{Output, Stdio};

    const MINIFIED: &[u8] = br#"{"z":1,"a":{"y":[1,2],"b":null},"m":"x"}"#;

    fn run(args: &[&str], input: &[u8]) -> Output {
        let mut child = view()
            .args(["--no-pager"])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }

    /// Texto sem as sequências de cor do `bat`.
    fn plain(out: &Output) -> String {
        let mut text = String::new();
        let mut chars = stdout_of(out).chars().collect::<Vec<_>>().into_iter();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                text.push(c);
            }
        }
        text
    }

    #[test]
    fn minified_json_is_formatted_with_key_order_kept() {
        let out = run(&[], MINIFIED);
        assert!(out.status.success(), "{out:?}");
        let expected = "{\n  \"z\": 1,\n  \"a\": {\n    \"y\": [\n      1,\n      2\n    ],\n    \"b\": null\n  },\n  \"m\": \"x\"\n}\n";
        assert_eq!(plain(&out), expected);
    }

    #[test]
    fn raw_keeps_the_original_text() {
        let out = run(&["--raw", "--as", "json"], MINIFIED);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(plain(&out).trim_end().as_bytes(), MINIFIED);
    }

    #[test]
    fn invalid_json_is_shown_as_is_without_error() {
        let out = run(&["--as", "json"], b"{\"a\": ");
        assert!(out.status.success(), "{out:?}");
        assert_eq!(plain(&out).trim_end(), "{\"a\":");
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn invalid_json_note_appears_with_verbose() {
        let out = run(&["-v", "--as", "json"], b"{\"a\": ");
        assert!(out.status.success(), "{out:?}");
        assert!(stdout_of(&out).contains("# json inválido: mostrado como está"));
        assert!(!stdout_of(&out).contains("interno"));
    }

    #[test]
    fn raw_with_another_format_is_a_usage_error() {
        let out = run(&["--raw", "--as", "yaml"], MINIFIED);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("--raw"));
    }

    #[test]
    fn json_lines_stay_as_they_are() {
        let dir = std::env::temp_dir().join(format!("view-jsonl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.jsonl");
        std::fs::write(&file, "{\"a\":1}\n").unwrap();
        let out = view().args(["--no-pager"]).arg(&file).output().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(out.status.success(), "{out:?}");
        assert_eq!(plain(&out).trim_end(), "{\"a\":1}");
    }
}

mod html {
    use super::*;
    use std::io::Write;
    use std::process::Stdio;

    const PAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/examples/page.html");

    /// Roda `view` com `args` e o `input` no stdin, se houver.
    fn run(args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let mut child = view()
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(bytes) = input {
            child.stdin.take().unwrap().write_all(bytes).unwrap();
        }
        child.wait_with_output().unwrap()
    }

    /// Remove sequências de escape ANSI (o `glow` colore até os espaços).
    fn strip_ansi(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn assert_markdown_of_page(text: &str) {
        let text = &strip_ansi(text);
        assert!(text.contains("Bem-vindo"), "{text}");
        assert!(text.contains("Primeiro item"), "{text}");
        assert!(text.contains("guia completo"), "{text}");
        assert!(text.contains("alfa"), "{text}");
        assert!(!text.contains("console.log"), "{text}");
        assert!(!text.contains("font-family"), "{text}");
        assert!(!text.contains("<h1>"), "{text}");
    }

    #[test]
    fn file_renders_as_markdown() {
        let out = run(&["--no-pager", PAGE], None);
        assert!(out.status.success(), "{out:?}");
        assert_markdown_of_page(&stdout_of(&out));
    }

    #[test]
    fn stdin_is_detected_as_html() {
        let page = std::fs::read(PAGE).unwrap();
        let out = run(&["--no-pager"], Some(&page));
        assert!(out.status.success(), "{out:?}");
        assert_markdown_of_page(&stdout_of(&out));
    }

    #[test]
    fn dry_run_shows_internal_step_and_markdown_viewer() {
        let out = run(&["-n", PAGE], None);
        let text = stdout_of(&out);
        assert!(text.contains("html→md interno"), "{text}");
        assert!(text.contains(" | "), "{text}");
        assert!(
            text.contains("glow") || text.contains("bat") || text.contains("cat"),
            "{text}"
        );
    }
}

mod csv {
    use super::*;

    /// Saída de `view --no-pager -q` sobre um exemplo.
    fn table_of(example: &str) -> String {
        let out = view()
            .args(["--no-pager", &format!("{EXAMPLES}/{example}")])
            .output()
            .unwrap();
        assert!(out.status.success());
        stdout_of(&out)
    }

    #[test]
    fn csv_is_a_table_with_the_quoted_comma_in_one_cell() {
        let text = table_of("table.csv");
        assert!(text.contains("┌"), "{text}");
        assert!(text.contains("Belo Horizonte, MG"), "{text}");
        // Cabeçalho + 4 linhas de dados.
        let row_lines = text.lines().filter(|l| l.starts_with('│')).count();
        assert_eq!(row_lines, 5);
    }

    #[test]
    fn tsv_is_a_table_split_on_tabs() {
        let text = table_of("table.tsv");
        assert!(text.contains("Belo Horizonte"), "{text}");
        assert!(text.contains("valor"));
        assert!(!text.contains('\t'));
    }

    #[test]
    fn header_says_csv_table_without_a_made_up_as() {
        let out = view()
            .args(["-n", &format!("{EXAMPLES}/table.csv")])
            .output()
            .unwrap();
        let text = stdout_of(&out);
        assert!(text.starts_with("$ view "), "{text}");
        assert!(!text.contains("--as"), "{text}");
        assert!(text.contains("# csv→tabela interno"), "{text}");
    }

    #[test]
    fn stdin_is_not_guessed_as_csv_without_as() {
        let out = view_stdin(&["--no-pager"], &example_bytes("table.csv"));
        assert!(!stdout_of(&out).contains('┌'));
    }

    #[test]
    fn stdin_with_as_csv_or_tsv_is_a_table() {
        let out = view_stdin(&["--no-pager", "--as", "csv"], &example_bytes("table.csv"));
        assert!(stdout_of(&out).contains("Belo Horizonte, MG"));
        let out = view_stdin(&["--no-pager", "--as", "csv"], &example_bytes("table.tsv"));
        let text = stdout_of(&out);
        assert!(
            text.contains("Porto Alegre") && !text.contains('\t'),
            "{text}"
        );
    }

    #[test]
    fn table_fits_the_terminal_width() {
        let long = format!("a,b\n{},{}\n", "x".repeat(200), "y".repeat(200));
        let out = view_stdin(&["--no-pager", "--as", "csv"], long.as_bytes());
        for line in stdout_of(&out).lines() {
            assert!(line.chars().count() <= 80, "{line}");
        }
    }

    #[test]
    fn more_than_10000_rows_are_cut_with_a_notice() {
        let mut data = String::from("n\n");
        for i in 0..10_005 {
            data.push_str(&format!("linha{i}\n"));
        }
        let out = view_stdin(&["--no-pager", "--as", "csv"], data.as_bytes());
        let text = stdout_of(&out);
        assert!(text.contains("linha9999 "), "fim das 10000 linhas");
        assert!(!text.contains("linha10000"));
        assert!(
            text.contains("10000 linhas mostradas, 5 omitidas"),
            "{}",
            text.lines().last().unwrap()
        );
    }

    #[test]
    fn exactly_10000_rows_have_no_notice() {
        let mut data = String::from("n\n");
        for i in 0..10_000 {
            data.push_str(&format!("{i}\n"));
        }
        let out = view_stdin(&["--no-pager", "--as", "csv"], data.as_bytes());
        assert!(!stdout_of(&out).contains("omitidas"));
    }
}

/// Testes da segunda leva com programas externos (`pdftotext`, `bsdtar`,
/// `ffprobe`). Sem o programa no PATH, o teste não tem o que verificar.
mod external_viewers {
    use super::*;

    fn installed(program: &str) -> bool {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .any(|dir| dir.join(program).is_file())
    }

    fn file_and_stdin(example: &str) -> [std::process::Output; 2] {
        [
            view_example(example, &["--no-pager"]),
            view_stdin(&["--no-pager"], &example_bytes(example)),
        ]
    }

    #[test]
    fn pdf_is_text() {
        if !installed("pdftotext") {
            return;
        }
        for out in file_and_stdin("doc.pdf") {
            assert!(out.status.success(), "{out:?}");
            assert!(stdout_of(&out).contains("Documento de exemplo"));
        }
    }

    #[test]
    fn pdf_dry_run_writes_text_to_stdout() {
        let text = stdout_of(&dry_run("doc.pdf"));
        assert!(text.starts_with("$ pdftotext -layout "), "{text}");
        assert!(text.trim_end().ends_with("doc.pdf -"), "{text}");
    }

    #[test]
    fn archives_are_listed() {
        if !installed("bsdtar") {
            return;
        }
        for (example, member) in [
            ("archive.zip", "notes.txt"),
            ("archive.tar.gz", "data.json"),
        ] {
            for out in file_and_stdin(example) {
                assert!(out.status.success(), "{example}: {out:?}");
                assert!(stdout_of(&out).contains(member), "{example}");
            }
        }
    }

    #[test]
    fn media_shows_ffprobe_summary_from_stderr() {
        if !installed("ffprobe") {
            return;
        }
        for out in file_and_stdin("sound.wav") {
            assert!(out.status.success(), "{out:?}");
            let text = stdout_of(&out);
            assert!(text.contains("Duration: 00:00:01.00"), "{text}");
            assert!(text.contains("Audio: pcm_s16le"), "{text}");
        }
        let text = stdout_of(&dry_run("sound.wav"));
        assert!(text.trim_end().ends_with("sound.wav 2>&1"), "{text}");
    }

    #[test]
    fn missing_viewer_error_suggests_install() {
        let out = view()
            .args(["--no-pager", &format!("{EXAMPLES}/doc.pdf")])
            .env("PATH", "/nonexistent")
            .output()
            .unwrap();
        assert!(!out.status.success());
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("pacman -S poppler"), "{err}");
    }
}

mod svg {
    use super::*;

    #[test]
    fn svg_becomes_half_blocks_from_file_and_stdin() {
        let from_stdin = view_stdin(&["-v"], &example_bytes("drawing.svg"));
        for out in [view_example("drawing.svg", &["-v"]), from_stdin] {
            assert!(out.status.success(), "{out:?}");
            let text = stdout_of(&out);
            assert!(text.contains("# imagem: meio-bloco"), "{text}");
            assert!(text.contains('▀'));
            // Fundo branco e o retângulo azul `#4a90d9`.
            assert!(text.contains("255;255;255"));
            assert!(text.contains("74;144;217"));
        }
    }

    #[test]
    fn svg_fits_the_default_width() {
        let text = stdout_of(&view_example("drawing.svg", &[]));
        let widest = text.lines().map(|l| l.matches('▀').count()).max();
        assert_eq!(widest, Some(80));
    }

    #[test]
    fn invalid_svg_is_shown_as_text() {
        let out = view_stdin(&["-v", "--as", "svg"], b"<svg><rect");
        assert!(out.status.success(), "{out:?}");
        let text = stdout_of(&out);
        assert!(
            text.contains("# svg inválido: mostrado como texto"),
            "{text}"
        );
        assert!(text.contains("rect"));
        assert!(!text.contains('▀'));
    }
}
