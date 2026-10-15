//! URLs http: servidor local em thread, sem rede externa.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Output};
use std::sync::OnceLock;

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/examples");

fn example(name: &str) -> Vec<u8> {
    std::fs::read(format!("{EXAMPLES}/{name}")).unwrap()
}

/// Resposta fixa para `path`: status, `Content-Type` (se houver), cabeçalhos
/// extras e corpo.
fn respond(path: &str) -> (u16, Option<&'static str>, Option<String>, Vec<u8>) {
    match path {
        "/page.html" => (
            200,
            Some("text/html; charset=utf-8"),
            None,
            example("page.html"),
        ),
        "/data.json" => (200, Some("application/json"), None, example("data.json")),
        "/table" => (
            200,
            Some("text/csv; charset=utf-8"),
            None,
            example("table.csv"),
        ),
        "/image.png" => (200, Some("image/png"), None, example("image.png")),
        // Sem tipo útil: o conteúdo decide.
        "/untyped-html" => (200, None, None, example("page.html")),
        "/plain-json" => (200, Some("text/plain"), None, example("data.json")),
        "/octet-png" => (
            200,
            Some("application/octet-stream"),
            None,
            example("image.png"),
        ),
        "/redirect" => (302, None, Some("Location: /page.html".into()), Vec::new()),
        _ => (404, Some("text/plain"), None, b"nada aqui".to_vec()),
    }
}

fn handle(mut stream: TcpStream) {
    let mut request = Vec::new();
    let mut buf = [0u8; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&buf[..n]),
        }
    }
    let request = String::from_utf8_lossy(&request);
    let path = request.split_whitespace().nth(1).unwrap_or("/");
    let (status, content_type, extra, body) = respond(path);
    let mut head = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\n", body.len());
    if let Some(content_type) = content_type {
        head.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    if let Some(extra) = extra {
        head.push_str(&format!("{extra}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
}

/// Endereço base (`http://127.0.0.1:porta`) do servidor local, iniciado uma vez.
fn base() -> &'static str {
    static BASE: OnceLock<String> = OnceLock::new();
    BASE.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || handle(stream));
            }
        });
        base
    })
}

fn view(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_view"))
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

fn fetch(path: &str, flags: &[&str]) -> Output {
    let url = format!("{}{path}", base());
    let mut args = vec!["--no-pager"];
    args.extend_from_slice(flags);
    args.push(&url);
    view(&args)
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Remove sequências de escape ANSI.
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

#[test]
fn html_is_rendered_as_markdown() {
    let out = fetch("/page.html", &[]);
    assert!(out.status.success(), "{out:?}");
    let text = strip_ansi(&stdout_of(&out));
    assert!(text.contains("Bem-vindo"), "{text}");
    assert!(text.contains("guia completo"), "{text}");
    assert!(!text.contains("console.log"), "{text}");
    assert!(!text.contains("<h1>"), "{text}");
}

#[test]
fn json_is_highlighted() {
    let out = fetch("/data.json", &[]);
    assert!(out.status.success(), "{out:?}");
    let text = strip_ansi(&stdout_of(&out));
    assert!(text.contains("\"projeto\""), "{text}");
}

#[test]
fn image_is_routed_to_the_image_chain() {
    // O tipo veio do `Content-Type`; com stdout num pipe, sai meio-bloco.
    let out = fetch("/image.png", &[]);
    assert!(out.status.success(), "{out:?}");
    assert!(stdout_of(&out).contains('▀'));
}

#[test]
fn missing_or_generic_content_type_falls_back_to_the_content() {
    let html = fetch("/untyped-html", &[]);
    assert!(html.status.success(), "{html:?}");
    assert!(strip_ansi(&stdout_of(&html)).contains("Bem-vindo"));

    let json = fetch("/plain-json", &[]);
    assert!(json.status.success(), "{json:?}");
    assert!(strip_ansi(&stdout_of(&json)).contains("\"projeto\""));

    let png = fetch("/octet-png", &[]);
    assert!(png.status.success(), "{png:?}");
    assert!(stdout_of(&png).contains('▀'));
}

#[test]
fn redirects_are_followed() {
    let out = fetch("/redirect", &[]);
    assert!(out.status.success(), "{out:?}");
    assert!(strip_ansi(&stdout_of(&out)).contains("Bem-vindo"));
}

#[test]
fn header_shows_curl_piped_into_the_backend() {
    let out = fetch("/data.json", &["-v"]);
    assert!(out.status.success(), "{out:?}");
    let text = stdout_of(&out);
    let first = text.lines().next().unwrap();
    assert!(
        first.starts_with(&format!("\x1b[2m$ curl -sL {}/data.json | ", base())),
        "{first}"
    );
}

#[test]
fn http_error_is_a_short_message_with_the_status() {
    let out = fetch("/nope", &[]);
    assert!(!out.status.success());
    assert!(stdout_of(&out).is_empty());
    let err = stderr_of(&out);
    assert!(err.contains("HTTP 404"), "{err}");
    assert!(err.contains("/nope"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
    assert_eq!(err.trim_end().lines().count(), 1, "{err}");
}

#[test]
fn network_error_is_a_short_message() {
    // Porta fechada: o listener é largado antes da requisição.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let out = view(&["--no-pager", &format!("http://127.0.0.1:{port}/")]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.starts_with("view: http://127.0.0.1:"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn dry_run_does_not_make_a_request() {
    // Porta fechada: qualquer requisição falharia.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let url = format!("http://127.0.0.1:{port}/page.html");
    let out = view(&["-n", &url]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(stdout_of(&out), format!("$ curl -sL {url} | view -\n"));

    let out = view(&["-n", "--as", "json", &url]);
    assert!(out.status.success(), "{out:?}");
    let text = stdout_of(&out);
    assert!(text.starts_with(&format!("$ curl -sL {url} | ")), "{text}");
    assert!(text.contains("bat") || text.contains("cat"), "{text}");
}

#[test]
fn dry_run_example_com_is_offline() {
    let out = view(&["-n", "https://example.com"]);
    assert!(out.status.success(), "{out:?}");
    assert!(stdout_of(&out).starts_with("$ curl -sL https://example.com | "));
}

#[test]
fn csv_content_type_is_rendered_as_a_table() {
    let out = fetch("/table", &[]);
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(
        text.contains("┌") && text.contains("Belo Horizonte, MG"),
        "{text}"
    );
}
