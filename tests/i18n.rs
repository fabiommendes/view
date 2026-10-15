//! Idioma das mensagens: escolha pelo ambiente, tradução e arquivos `.ftl`.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixture.txt");
const LOCALES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/locales");

/// Ambiente de idioma: `(LC_ALL, LC_MESSAGES, LANG)`. `None` deixa a variável
/// fora do ambiente do processo.
type Env<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);

fn view_with(program: &Path, env: Env) -> Command {
    let mut command = Command::new(program);
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        command.env_remove(name);
    }
    let (all, messages, lang) = env;
    for (name, value) in [("LC_ALL", all), ("LC_MESSAGES", messages), ("LANG", lang)] {
        if let Some(value) = value {
            command.env(name, value);
        }
    }
    command
}

fn view(env: Env) -> Command {
    view_with(Path::new(env!("CARGO_BIN_EXE_view")), env)
}

fn all(value: &str) -> Env<'_> {
    (Some(value), None, None)
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Erro do formato desconhecido: a mensagem mais simples de provocar.
fn unknown_format_error(env: Env) -> String {
    let out = view(env).args(["--as", "nope", FIXTURE]).output().unwrap();
    assert!(!out.status.success());
    stderr(&out)
}

fn run_stdin(env: Env, args: &[&str], bytes: &[u8]) -> Output {
    let mut child = view(env)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    child.wait_with_output().unwrap()
}

const EN: &str = "unknown format: nope";
const PT: &str = "formato desconhecido: nope";
const ES: &str = "formato desconocido: nope";

// Escolha do idioma (R4, R5)

#[test]
fn lc_all_selects_the_language() {
    assert!(unknown_format_error(all("en_US.UTF-8")).contains(EN));
    assert!(unknown_format_error(all("pt_BR.UTF-8")).contains(PT));
    assert!(unknown_format_error(all("es_ES.UTF-8")).contains(ES));
}

#[test]
fn lc_all_wins_over_lc_messages_and_lang() {
    let err = unknown_format_error((
        Some("en_US.UTF-8"),
        Some("pt_BR.UTF-8"),
        Some("es_ES.UTF-8"),
    ));
    assert!(err.contains(EN), "{err}");
}

#[test]
fn lc_messages_wins_over_lang() {
    let err = unknown_format_error((None, Some("pt_BR.UTF-8"), Some("es_ES.UTF-8")));
    assert!(err.contains(PT), "{err}");
}

#[test]
fn lang_is_used_when_nothing_else_is_set() {
    let err = unknown_format_error((None, None, Some("es_ES.UTF-8")));
    assert!(err.contains(ES), "{err}");
}

#[test]
fn empty_variables_count_as_unset() {
    let err = unknown_format_error((Some(""), Some(""), Some("es_ES.UTF-8")));
    assert!(err.contains(ES), "{err}");
}

#[test]
fn regional_variants_fall_back_to_the_language() {
    for (value, expected) in [
        ("pt", PT),
        ("pt_PT.UTF-8", PT),
        ("pt_BR", PT),
        ("es", ES),
        ("es_AR.UTF-8", ES),
        ("en_GB.UTF-8", EN),
    ] {
        let err = unknown_format_error(all(value));
        assert!(err.contains(expected), "{value}: {err}");
    }
}

#[test]
fn unknown_unset_and_posix_locales_fall_back_to_english() {
    for env in [
        all("de_DE.UTF-8"),
        all("C"),
        all("POSIX"),
        all("C.UTF-8"),
        (None, None, None),
    ] {
        let err = unknown_format_error(env);
        assert!(err.contains(EN), "{env:?}: {err}");
    }
}

#[test]
fn messages_have_no_bidi_isolation_marks() {
    for lang in ["en_US.UTF-8", "pt_BR.UTF-8", "es_ES.UTF-8"] {
        let err = unknown_format_error(all(lang));
        assert!(!err.contains(['\u{2068}', '\u{2069}']), "{lang}: {err:?}");
    }
}

// Mensagens (R7, R9)

#[test]
fn raw_with_another_format_error_is_translated() {
    for (lang, expected) in [
        (
            "en_US.UTF-8",
            "--raw only applies to json, but --as asks for yaml",
        ),
        ("pt_BR.UTF-8", "--raw só vale para json, mas --as pede yaml"),
        (
            "es_ES.UTF-8",
            "--raw solo vale para json, pero --as pide yaml",
        ),
    ] {
        let out = run_stdin(all(lang), &["--raw", "--as", "yaml"], b"{}");
        assert!(!out.status.success());
        assert!(stderr(&out).contains(expected), "{lang}: {}", stderr(&out));
    }
}

#[test]
fn image_decode_error_is_translated() {
    for (lang, expected) in [
        ("en_US.UTF-8", "could not decode the image"),
        ("pt_BR.UTF-8", "não foi possível decodificar a imagem"),
        ("es_ES.UTF-8", "no se pudo decodificar la imagen"),
    ] {
        let out = run_stdin(all(lang), &["--as", "image"], b"not an image");
        assert!(!out.status.success());
        assert!(stderr(&out).contains(expected), "{lang}: {}", stderr(&out));
    }
}

#[test]
fn invalid_json_note_is_translated() {
    for (lang, expected) in [
        ("en_US.UTF-8", "# invalid json: shown as is"),
        ("pt_BR.UTF-8", "# json inválido: mostrado como está"),
        ("es_ES.UTF-8", "# json inválido: se muestra tal cual"),
    ] {
        let out = run_stdin(
            all(lang),
            &["-v", "--no-pager", "--as", "json"],
            b"{\"a\": ",
        );
        assert!(out.status.success(), "{out:?}");
        assert!(stdout(&out).contains(expected), "{lang}: {}", stdout(&out));
    }
}

#[test]
fn invalid_svg_note_is_translated() {
    for (lang, expected) in [
        ("en_US.UTF-8", "# invalid svg: shown as text"),
        ("pt_BR.UTF-8", "# svg inválido: mostrado como texto"),
        ("es_ES.UTF-8", "# svg inválido: se muestra como texto"),
    ] {
        let out = run_stdin(
            all(lang),
            &["-v", "--no-pager", "--as", "svg"],
            b"<svg><rect",
        );
        assert!(out.status.success(), "{out:?}");
        assert!(stdout(&out).contains(expected), "{lang}: {}", stdout(&out));
    }
}

#[test]
fn equivalent_command_does_not_depend_on_the_language() {
    let command = |lang| {
        let out = view(all(lang)).args(["-n", FIXTURE]).output().unwrap();
        assert!(out.status.success());
        stdout(&out)
    };
    let english = command("en_US.UTF-8");
    assert!(english.starts_with("$ "));
    assert_eq!(command("pt_BR.UTF-8"), english);
    assert_eq!(command("es_ES.UTF-8"), english);
}

// --help (R6)

fn help(lang: &str) -> String {
    let out = view(all(lang)).arg("--help").output().unwrap();
    assert!(out.status.success());
    stdout(&out)
}

#[test]
fn help_is_translated() {
    let english = help("en_US.UTF-8");
    assert!(english.contains("Show a file, URL or stdin with the right viewer"));
    assert!(english.contains("Only print the command that would run."));

    let portuguese = help("pt_BR.UTF-8");
    assert!(portuguese.contains("Mostra um arquivo, URL ou stdin com o visualizador certo"));
    assert!(portuguese.contains("Só imprime o comando que rodaria."));
    assert!(!portuguese.contains("Only print"));

    let spanish = help("es_ES.UTF-8");
    assert!(spanish.contains("Muestra un archivo, URL o stdin con el visor adecuado"));
    assert!(spanish.contains("Solo imprime el comando que se ejecutaría."));
    assert!(!spanish.contains("Only print"));
}

#[test]
fn help_still_lists_every_option() {
    for lang in ["en_US.UTF-8", "pt_BR.UTF-8", "es_ES.UTF-8"] {
        let text = help(lang);
        for flag in ["--verbose", "--dry-run", "--as", "--raw", "--no-pager"] {
            assert!(text.contains(flag), "{lang}: {flag} em\n{text}");
        }
    }
}

// Arquivos de tradução (R1, R3)

/// Identificadores das mensagens de um `.ftl`: linhas `chave = ...` sem recuo.
fn ftl_keys(path: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let (key, _) = line.split_once('=')?;
            let key = key.trim_end();
            let valid = !key.is_empty()
                && key.starts_with(|c: char| c.is_ascii_alphabetic())
                && !line.starts_with(char::is_whitespace)
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            valid.then(|| key.to_owned())
        })
        .collect()
}

#[test]
fn every_language_has_the_same_message_keys() {
    let keys = |tag: &str| ftl_keys(&Path::new(LOCALES).join(tag).join("view.ftl"));
    let english = keys("en");
    assert!(english.len() >= 5, "poucas chaves em en: {english:?}");
    for tag in ["pt-BR", "es"] {
        assert_eq!(keys(tag), english, "chaves de {tag} diferem de en");
    }
}

#[test]
fn every_locale_directory_has_the_same_keys_as_english() {
    let english = ftl_keys(&Path::new(LOCALES).join("en").join("view.ftl"));
    for entry in std::fs::read_dir(LOCALES).unwrap() {
        let dir = entry.unwrap().path();
        if dir.is_dir() {
            assert_eq!(ftl_keys(&dir.join("view.ftl")), english, "{dir:?}");
        }
    }
}

#[test]
fn translations_are_embedded_in_the_binary() {
    let dir = std::env::temp_dir().join(format!("view-i18n-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let copy = dir.join("view");
    std::fs::copy(env!("CARGO_BIN_EXE_view"), &copy).unwrap();
    let out = view_with(&copy, all("es_ES.UTF-8"))
        .current_dir(&dir)
        .args(["--as", "nope", FIXTURE])
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(stderr(&out).contains(ES), "{}", stderr(&out));
}
