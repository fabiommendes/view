//! Cabeçalho: comando equivalente, dica de instalação e régua.

use std::path::Path;

use crate::backend::{Backend, Internal, Resolution};
use crate::detect::{Input, Kind};

const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// Largura do terminal em colunas; 80 quando não há terminal.
pub fn terminal_width() -> u16 {
    terminal_size::terminal_size().map_or(80, |(w, _)| w.0)
}

/// Linha `$ ...` que reproduz o que foi feito. `stdin_path` é o arquivo
/// temporário usado por backends que exigem caminho quando a entrada é stdin.
/// `forced` é o formato passado pelo usuário em `--as`, se houve: só ele
/// aparece como `--as` no comando de backends internos.
pub fn command_line(
    backend: &Backend,
    input: &Input,
    forced: Option<Kind>,
    stdin_path: &Path,
) -> String {
    format!("$ {}", command_body(backend, input, forced, stdin_path))
}

/// Comando de uma URL baixada: o `curl` equivalente alimenta o comando do
/// backend, que lê o conteúdo pelo stdin. `command` é o resultado de
/// `command_line` para a entrada `Input::Stdin`.
pub fn from_url(url: &str, command: &str) -> String {
    let body = command.strip_prefix("$ ").unwrap_or(command);
    format!("$ curl -sL {} | {body}", shell_quote(url))
}

/// Comando de uma URL ainda não baixada (`-n` não faz requisição), então o
/// backend é desconhecido.
pub fn from_url_unresolved(url: &str) -> String {
    format!("$ curl -sL {} | view -", shell_quote(url))
}

/// Comando sem o prefixo `$ `. Num `Pipe`, o destino lê o que o renderizador
/// interno produz, então o cabeçalho mostra o destino sem alvo.
fn command_body(
    backend: &Backend,
    input: &Input,
    forced: Option<Kind>,
    stdin_path: &Path,
) -> String {
    match backend {
        Backend::External { .. } => {
            let argv = backend
                .argv(input, stdin_path)
                .expect("backend externo sempre tem argv");
            shell_words(&argv)
        }
        Backend::Internal(internal) => internal_body(*internal, input, forced),
        Backend::Pipe { internal, sink } => {
            let sink = match sink.as_ref() {
                Backend::External { .. } => {
                    let mut argv = sink
                        .argv(&Input::Stdin, stdin_path)
                        .expect("backend externo sempre tem argv");
                    if !sink.needs_path() {
                        argv.pop();
                    }
                    shell_words(&argv)
                }
                other => command_body(other, &Input::Stdin, forced, stdin_path),
            };
            format!("{} | {sink}", internal_body(*internal, input, forced))
        }
    }
}

fn internal_body(internal: Internal, input: &Input, forced: Option<Kind>) -> String {
    let target = match input {
        Input::Stdin => "-".to_owned(),
        Input::Target(t) => shell_quote(t),
    };
    let as_option = forced.map_or_else(String::new, |kind| format!("--as {} ", kind.name()));
    format!("view {as_option}{target}  # {}", internal.description())
}

fn shell_words(argv: &[String]) -> String {
    let words: Vec<String> = argv.iter().map(|a| shell_quote(a)).collect();
    words.join(" ")
}

/// Sugestão de instalação quando a cadeia caiu num fallback.
pub fn install_hint(resolution: &Resolution<'_>) -> Option<String> {
    resolution.skipped.iter().find_map(|b| match b {
        Backend::External {
            program,
            install_hint: Some(hint),
            ..
        } => Some(format!("# {program} não encontrado: {hint}")),
        _ => None,
    })
}

/// Monta o cabeçalho completo, esmaecido, terminado em newline. Cada nota
/// ocupa uma linha entre o comando e a régua.
pub fn render(command: &str, notes: &[String], width: usize) -> String {
    let mut out = format!("{DIM}{command}{RESET}\n");
    for note in notes {
        out.push_str(&format!("{DIM}{note}{RESET}\n"));
    }
    out.push_str(&format!("{DIM}{}{RESET}\n", "─".repeat(width)));
    out
}

fn shell_quote(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:,@%+".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_has_command_ruler_and_dim() {
        let out = render("$ cat a", &[], 5);
        assert_eq!(out, "\x1b[2m$ cat a\x1b[0m\n\x1b[2m─────\x1b[0m\n");
    }

    #[test]
    fn render_places_hint_between_command_and_ruler() {
        let out = render(
            "$ bat a",
            &["# glow não encontrado: pacman -S glow".to_owned()],
            3,
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[1].contains("glow não encontrado: pacman -S glow"));
        assert!(lines[2].contains("───"));
    }

    #[test]
    fn command_line_quotes_words() {
        let backend = Backend::external("bat", &["-l", "md"], None);
        let input = Input::Target("meu arquivo.md".into());
        assert_eq!(
            command_line(&backend, &input, None, Path::new("/tmp/x")),
            "$ bat -l md 'meu arquivo.md'"
        );
    }

    #[test]
    fn internal_command_line_describes_work() {
        let backend = Backend::Internal(Internal::Csv);
        let line = command_line(
            &backend,
            &Input::Target("a.csv".into()),
            None,
            Path::new("/tmp/x"),
        );
        assert_eq!(line, "$ view a.csv  # csv→tabela interno");
    }

    #[test]
    fn as_option_appears_only_when_the_user_passed_it() {
        let backend = Backend::Pipe {
            internal: Internal::Html,
            sink: Box::new(Backend::external("glow", &[], None)),
        };
        let input = Input::Target("page.html".into());
        let tmp = Path::new("/tmp/x");
        assert_eq!(
            command_line(&backend, &input, None, tmp),
            "$ view page.html  # html→md interno | glow"
        );
        assert_eq!(
            command_line(&backend, &input, Some(Kind::Html), tmp),
            "$ view --as html page.html  # html→md interno | glow"
        );
    }

    #[test]
    fn pipe_command_line_shows_internal_step_then_sink_without_target() {
        use crate::convert::Format;
        let backend = Backend::Pipe {
            internal: Internal::Convert {
                from: Format::Json,
                to: Format::Yaml,
            },
            sink: Box::new(Backend::external("bat", &["-l", "yaml"], None)),
        };
        let line = command_line(
            &backend,
            &Input::Target("foo.json".into()),
            Some(Kind::Yaml),
            Path::new("/tmp/x"),
        );
        assert_eq!(
            line,
            "$ view --as yaml foo.json  # json→yaml interno | bat -l yaml"
        );
    }

    #[test]
    fn stdin_command_line_shows_dash_or_temp_path() {
        let tmp = Path::new("/tmp/view-1");
        let bat = Backend::external("bat", &[], None);
        assert_eq!(command_line(&bat, &Input::Stdin, None, tmp), "$ bat -");
        let ls = Backend::external("ls", &["-la"], None).path_only();
        assert_eq!(
            command_line(&ls, &Input::Stdin, None, tmp),
            "$ ls -la /tmp/view-1"
        );
    }

    #[test]
    fn hint_comes_from_preferred_backend() {
        let chain = [
            Backend::external("glow", &[], Some("pacman -S glow")),
            Backend::external("cat", &[], None),
        ];
        let res = Resolution {
            backend: &chain[1],
            skipped: &chain[..1],
        };
        assert_eq!(
            install_hint(&res).unwrap(),
            "# glow não encontrado: pacman -S glow"
        );
        let res = Resolution {
            backend: &chain[0],
            skipped: &[],
        };
        assert!(install_hint(&res).is_none());
    }
}
