//! Detecção do tipo de conteúdo a partir do alvo.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Result, bail};

/// Alvo a visualizar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Stdin,
    /// Caminho ou URL, como veio da linha de comando.
    Target(String),
}

impl Input {
    /// `None` e `-` significam stdin.
    pub fn from_arg(arg: Option<&str>) -> Self {
        match arg {
            None | Some("-") => Self::Stdin,
            Some(target) => Self::Target(target.to_owned()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Code,
    Markdown,
    Html,
    Json,
    Yaml,
    Toml,
    Image,
    Svg,
    Csv,
    Pdf,
    Archive,
    Directory,
    Binary,
    Media,
    Text,
}

impl Kind {
    /// Nome canônico, o mesmo aceito por `--as`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Image => "image",
            Self::Svg => "svg",
            Self::Csv => "csv",
            Self::Pdf => "pdf",
            Self::Archive => "archive",
            Self::Directory => "dir",
            Self::Binary => "binary",
            Self::Media => "media",
            Self::Text => "text",
        }
    }

    /// Interpreta o argumento de `--as`.
    pub fn from_name(name: &str) -> Result<Self> {
        Ok(match name.to_ascii_lowercase().as_str() {
            "code" => Self::Code,
            "md" | "markdown" => Self::Markdown,
            "html" => Self::Html,
            "json" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "image" | "img" => Self::Image,
            "svg" => Self::Svg,
            "csv" | "tsv" => Self::Csv,
            "pdf" => Self::Pdf,
            "archive" => Self::Archive,
            "dir" | "directory" => Self::Directory,
            "binary" | "bin" => Self::Binary,
            "media" => Self::Media,
            "text" | "txt" => Self::Text,
            other => bail!("formato desconhecido: {other}"),
        })
    }
}

/// Quantos bytes do início do arquivo a detecção por conteúdo examina.
const SNIFF_LEN: usize = 8192;

/// Detecta o tipo, na ordem: diretório, magic bytes, extensão e, por
/// último, texto ou binário pelo conteúdo.
pub fn detect(input: &Input) -> Kind {
    match input {
        Input::Stdin => Kind::Text,
        Input::Target(target) => {
            if Path::new(target).is_dir() {
                Kind::Directory
            } else {
                detect_file(Path::new(target))
            }
        }
    }
}

/// Detecta o tipo do conteúdo lido do stdin: magic bytes e, depois, texto ou
/// binário. Não há nome, então a extensão não participa; o `bat` também não
/// tem o que usar para escolher a sintaxe, então um texto que parece JSON ou
/// TOML vira `Json` ou `Toml`.
pub fn detect_bytes(bytes: &[u8]) -> Kind {
    let head = &bytes[..bytes.len().min(SNIFF_LEN)];
    match from_magic(head).unwrap_or_else(|| from_content(head)) {
        Kind::Text if looks_like_html(head) => Kind::Html,
        Kind::Text if looks_like_svg(head) => Kind::Svg,
        Kind::Text => sniff_structured(head).unwrap_or(Kind::Text),
        kind => kind,
    }
}

/// HTML começa (depois de espaços) com `<!doctype html` ou `<html`, sem
/// diferenciar maiúsculas.
fn looks_like_html(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    let start: String = text
        .trim_start()
        .chars()
        .take(14)
        .collect::<String>()
        .to_ascii_lowercase();
    start.starts_with("<!doctype html") || start.starts_with("<html")
}

/// SVG começa (depois de espaços) com `<svg`, ou com `<?xml` e tem `<svg`
/// na amostra.
fn looks_like_svg(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    let start = text.trim_start();
    start.starts_with("<svg") || (start.starts_with("<?xml") && start.contains("<svg"))
}

/// Heurística barata, sem parser: JSON começa com `{` ou com `[` seguido de
/// um valor; TOML começa com um cabeçalho `[tabela]` ou tem só linhas
/// `chave = valor`, cabeçalhos e comentários.
fn sniff_structured(head: &[u8]) -> Option<Kind> {
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start();
    let mut chars = trimmed.chars();
    match chars.next()? {
        '{' => return Some(Kind::Json),
        '[' => {
            let next = chars.find(|c| !c.is_whitespace())?;
            if matches!(next, '"' | '{' | '[' | '-' | ']') || next.is_ascii_digit() {
                return Some(Kind::Json);
            }
        }
        _ => {}
    }
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    let first = lines.next()?;
    let is_header = |l: &str| {
        let name = l.trim_start_matches('[').trim_end_matches(']');
        l.starts_with('[')
            && l.ends_with(']')
            && !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-. \"'".contains(c))
    };
    let is_pair = |l: &str| {
        l.split_once('=').is_some_and(|(key, _)| {
            let key = key.trim();
            !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-.\"'".contains(c))
        })
    };
    let toml = is_header(first) || (is_pair(first) && lines.all(|l| is_header(l) || is_pair(l)));
    toml.then_some(Kind::Toml)
}

/// Detecção de um arquivo. Um arquivo ilegível ou inexistente só tem a
/// extensão para decidir e, sem ela, vira `Text`.
fn detect_file(path: &Path) -> Kind {
    let head = read_head(path).unwrap_or_default();
    from_magic(&head)
        .or_else(|| from_extension(path))
        .unwrap_or_else(|| from_content(&head))
}

/// Lê até `SNIFF_LEN` bytes do início do arquivo.
fn read_head(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(SNIFF_LEN);
    File::open(path)?
        .take(SNIFF_LEN as u64)
        .read_to_end(&mut head)?;
    Ok(head)
}

pub fn is_url(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// Tipo pelos magic bytes (`infer`). `None` se a assinatura não é conhecida
/// ou se é de texto (shebang), que a extensão e o conteúdo decidem melhor.
pub fn from_magic(head: &[u8]) -> Option<Kind> {
    let known = infer::get(head)?;
    if known.mime_type() == "application/pdf" {
        return Some(Kind::Pdf);
    }
    match known.matcher_type() {
        infer::MatcherType::Image => Some(Kind::Image),
        infer::MatcherType::Archive => Some(Kind::Archive),
        infer::MatcherType::Audio | infer::MatcherType::Video => Some(Kind::Media),
        infer::MatcherType::App
        | infer::MatcherType::Doc
        | infer::MatcherType::Book
        | infer::MatcherType::Font => Some(Kind::Binary),
        infer::MatcherType::Text | infer::MatcherType::Custom => None,
    }
}

/// Tipo pela extensão: a tabela própria primeiro, depois `mime_guess`.
/// `None` sem extensão ou com extensão desconhecida.
pub fn from_extension(path: &Path) -> Option<Kind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    from_extension_table(&ext).or_else(|| from_mime(path))
}

fn from_extension_table(ext: &str) -> Option<Kind> {
    Some(match ext {
        "md" | "markdown" => Kind::Markdown,
        "html" | "htm" => Kind::Html,
        "json" => Kind::Json,
        "yaml" | "yml" => Kind::Yaml,
        "toml" => Kind::Toml,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" => Kind::Image,
        "svg" => Kind::Svg,
        "csv" | "tsv" => Kind::Csv,
        "pdf" => Kind::Pdf,
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "7z" => Kind::Archive,
        "bin" | "exe" | "dll" | "so" | "o" | "a" => Kind::Binary,
        "mp3" | "mp4" | "mkv" | "webm" | "flac" | "wav" | "ogg" | "mov" => Kind::Media,
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "h" | "cc" | "cpp" | "hpp" | "go"
        | "java" | "kt" | "rb" | "lua" | "sh" | "bash" | "zsh" | "fish" | "css" | "sql" | "php"
        | "swift" | "zig" | "nix" => Kind::Code,
        _ => return None,
    })
}

/// Tipo pelo cabeçalho `Content-Type` de uma resposta HTTP. `None` se o
/// cabeçalho é ilegível ou genérico (`text/plain`, `application/octet-stream`
/// e tipos desconhecidos), caso em que o conteúdo decide.
pub fn from_content_type(value: &str) -> Option<Kind> {
    let essence = value.split(';').next()?.trim().to_ascii_lowercase();
    let (kind, subtype) = essence.split_once('/')?;
    let structured = subtype
        .rsplit_once('+')
        .map_or(subtype, |(_, suffix)| suffix);
    match (kind, structured) {
        ("text", "html") | ("application", "xhtml") => Some(Kind::Html),
        ("text", "csv" | "tab-separated-values") => Some(Kind::Csv),
        ("text", "markdown" | "x-markdown") => Some(Kind::Markdown),
        ("text", "css" | "javascript") | ("application", "javascript") => Some(Kind::Code),
        (_, "json") => Some(Kind::Json),
        (_, "yaml" | "x-yaml") => Some(Kind::Yaml),
        (_, "toml") => Some(Kind::Toml),
        ("image", _) if subtype.starts_with("svg") => Some(Kind::Svg),
        ("image", _) => Some(Kind::Image),
        ("audio" | "video", _) => Some(Kind::Media),
        ("application", "pdf") => Some(Kind::Pdf),
        ("application", sub) if is_archive_subtype(sub) => Some(Kind::Archive),
        _ => None,
    }
}

/// Tipo pelo MIME que `mime_guess` associa à extensão.
fn from_mime(path: &Path) -> Option<Kind> {
    let mime = mime_guess::from_path(path).first()?;
    match (mime.type_(), mime.subtype()) {
        (mime_guess::mime::IMAGE, mime_guess::mime::SVG) => Some(Kind::Svg),
        (mime_guess::mime::IMAGE, _) => Some(Kind::Image),
        (mime_guess::mime::AUDIO | mime_guess::mime::VIDEO, _) => Some(Kind::Media),
        (mime_guess::mime::TEXT, mime_guess::mime::HTML) => Some(Kind::Html),
        (mime_guess::mime::TEXT, mime_guess::mime::CSV) => Some(Kind::Csv),
        (mime_guess::mime::TEXT, _) => Some(Kind::Text),
        (mime_guess::mime::APPLICATION, mime_guess::mime::JSON) => Some(Kind::Json),
        (mime_guess::mime::APPLICATION, mime_guess::mime::PDF) => Some(Kind::Pdf),
        (mime_guess::mime::APPLICATION, mime_guess::mime::OCTET_STREAM) => Some(Kind::Binary),
        (mime_guess::mime::APPLICATION, sub) if is_archive_subtype(sub.as_str()) => {
            Some(Kind::Archive)
        }
        _ => None,
    }
}

fn is_archive_subtype(subtype: &str) -> bool {
    matches!(
        subtype,
        "zip" | "gzip" | "x-tar" | "x-bzip2" | "x-xz" | "zstd" | "x-7z-compressed"
    )
}

/// `Text` se o início do conteúdo é UTF-8 válido sem bytes nulos; senão
/// `Binary`. Uma sequência UTF-8 cortada no fim da amostra ainda é texto.
pub fn from_content(head: &[u8]) -> Kind {
    if head.contains(&0) {
        return Kind::Binary;
    }
    match std::str::from_utf8(head) {
        Err(e) if e.error_len().is_some() => Kind::Binary,
        _ => Kind::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/examples");

    fn example(name: &str) -> Input {
        Input::Target(format!("{EXAMPLES}/{name}"))
    }

    /// Cria um arquivo temporário com o nome e o conteúdo dados.
    fn temp_file(name: &str, content: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("view-detect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn kind_of(path: &Path) -> Kind {
        detect(&Input::Target(path.to_str().unwrap().into()))
    }

    #[test]
    fn extension_maps_to_kind() {
        let kind = |p| from_extension(Path::new(p));
        assert_eq!(kind("a/b/README.md"), Some(Kind::Markdown));
        assert_eq!(kind("main.RS"), Some(Kind::Code));
        assert_eq!(kind("x.tar"), Some(Kind::Archive));
        assert_eq!(kind("foto.jpeg"), Some(Kind::Image));
        assert_eq!(kind("data.yml"), Some(Kind::Yaml));
        assert_eq!(kind("notes.txt"), Some(Kind::Text));
    }

    #[test]
    fn mime_guess_covers_extensions_outside_the_table() {
        let kind = |p| from_extension(Path::new(p));
        assert_eq!(kind("a.tiff"), Some(Kind::Image));
        assert_eq!(kind("a.aac"), Some(Kind::Media));
        assert_eq!(kind("a.log"), Some(Kind::Text));
    }

    #[test]
    fn unknown_or_missing_extension_is_none() {
        assert_eq!(from_extension(Path::new("Makefile")), None);
        assert_eq!(from_extension(Path::new("a.zzzz")), None);
    }

    /// Cabeçalho ELF real: `infer` exige o cabeçalho inteiro.
    fn elf_head() -> Vec<u8> {
        let mut head = std::fs::read(format!("{EXAMPLES}/program")).unwrap();
        head.truncate(64);
        head
    }

    #[test]
    fn magic_bytes_map_to_kind() {
        assert_eq!(from_magic(b"\x89PNG\r\n\x1a\n"), Some(Kind::Image));
        assert_eq!(from_magic(&elf_head()), Some(Kind::Binary));
        assert_eq!(from_magic(b"%PDF-1.7\n"), Some(Kind::Pdf));
        assert_eq!(
            from_magic(b"PK\x03\x04\x14\x00\x00\x00\x08\x00"),
            Some(Kind::Archive)
        );
        assert_eq!(from_magic(b"\x1f\x8b\x08\x00"), Some(Kind::Archive));
    }

    #[test]
    fn unknown_or_text_magic_is_none() {
        assert_eq!(from_magic(b""), None);
        assert_eq!(from_magic("só texto\n".as_bytes()), None);
        assert_eq!(from_magic(b"#!/bin/sh\necho oi\n"), None);
    }

    #[test]
    fn content_text_or_binary() {
        assert_eq!(from_content(b""), Kind::Text);
        assert_eq!(from_content("acentuação\n".as_bytes()), Kind::Text);
        assert_eq!(from_content(b"abc\0def"), Kind::Binary);
        assert_eq!(from_content(b"abc\xff\xfedef"), Kind::Binary);
    }

    #[test]
    fn content_cut_in_the_middle_of_a_char_is_text() {
        let cut = &"é".as_bytes()[..1];
        assert_eq!(from_content(cut), Kind::Text);
    }

    #[test]
    fn magic_bytes_win_over_extension() {
        assert_eq!(detect(&example("image-as.txt")), Kind::Image);
        let path = temp_file("fake.md", &elf_head());
        assert_eq!(kind_of(&path), Kind::Binary);
    }

    #[test]
    fn extension_wins_over_content() {
        let path = temp_file("nulls.md", b"# titulo\0\0");
        assert_eq!(kind_of(&path), Kind::Markdown);
        assert_eq!(detect(&example("code.rs")), Kind::Code);
    }

    #[test]
    fn unknown_magic_and_extension_fall_back_to_content() {
        assert_eq!(detect(&example("LICENSE")), Kind::Text);
        let text = temp_file("noext-text", "olá\n".as_bytes());
        assert_eq!(kind_of(&text), Kind::Text);
        let binary = temp_file("noext-binary", b"\x01\x02\0\x03");
        assert_eq!(kind_of(&binary), Kind::Binary);
        let unknown_ext = temp_file("data.zzzz", b"\x01\0\x02");
        assert_eq!(kind_of(&unknown_ext), Kind::Binary);
    }

    #[test]
    fn only_the_first_8_kib_are_inspected() {
        let mut bytes = vec![b'a'; SNIFF_LEN];
        bytes.push(0);
        let path = temp_file("late-null", &bytes);
        assert_eq!(kind_of(&path), Kind::Text);
    }

    #[test]
    fn missing_file_uses_extension_then_text() {
        assert_eq!(
            detect(&Input::Target("/nao/existe/a.json".into())),
            Kind::Json
        );
        assert_eq!(detect(&Input::Target("/nao/existe/a".into())), Kind::Text);
    }

    #[test]
    fn examples_are_detected_by_content() {
        assert_eq!(detect(&example("program")), Kind::Binary);
        assert_eq!(detect(&example("binary.bin")), Kind::Binary);
        assert_eq!(detect(&example("notes.txt")), Kind::Text);
        assert_eq!(detect(&example("doc.pdf")), Kind::Pdf);
        assert_eq!(detect(&example("archive.zip")), Kind::Archive);
        assert_eq!(detect(&example("archive.tar.gz")), Kind::Archive);
        assert_eq!(detect(&example("image.png")), Kind::Image);
        assert_eq!(detect(&example("dir")), Kind::Directory);
    }

    #[test]
    fn url_prefix_is_http_or_https() {
        assert!(is_url("https://example.com/a.md"));
        assert!(!is_url("a.md"));
        assert!(!is_url("ftp://example.com"));
    }

    #[test]
    fn directory_is_detected() {
        let input = Input::Target(env!("CARGO_MANIFEST_DIR").into());
        assert_eq!(detect(&input), Kind::Directory);
    }

    #[test]
    fn bytes_use_magic_then_content() {
        let png = std::fs::read(format!("{EXAMPLES}/image.png")).unwrap();
        assert_eq!(detect_bytes(&png), Kind::Image);
        assert_eq!(detect_bytes(&elf_head()), Kind::Binary);
        assert_eq!(detect_bytes("# olá\n".as_bytes()), Kind::Text);
        assert_eq!(detect_bytes(b"\x01\0\x02"), Kind::Binary);
        assert_eq!(detect_bytes(b""), Kind::Text);
    }

    #[test]
    fn text_bytes_that_look_structured_are_sniffed() {
        let toml = std::fs::read(format!("{EXAMPLES}/config.toml")).unwrap();
        assert_eq!(detect_bytes(&toml), Kind::Toml);
        assert_eq!(detect_bytes(b"[package]\nname = \"a\"\n"), Kind::Toml);
        assert_eq!(detect_bytes(b"# c\na = 1\nb = \"x\"\n"), Kind::Toml);
        let json = std::fs::read(format!("{EXAMPLES}/data.json")).unwrap();
        assert_eq!(detect_bytes(&json), Kind::Json);
        assert_eq!(detect_bytes(b"  [1, 2]\n"), Kind::Json);
        assert_eq!(detect_bytes(b"[\"a\"]"), Kind::Json);
    }

    #[test]
    fn html_bytes_are_sniffed_by_the_start() {
        let page = example_bytes_text("page.html");
        assert_eq!(detect_bytes(&page), Kind::Html);
        assert_eq!(detect_bytes(b"\n  <HTML lang=pt>"), Kind::Html);
        assert_eq!(detect_bytes(b"<!DocType HTML>"), Kind::Html);
        assert_eq!(detect_bytes(b"<p>solto</p>"), Kind::Text);
        assert_eq!(detect_bytes(b"texto <html> no meio"), Kind::Text);
    }

    #[test]
    fn stdin_svg_is_detected() {
        let drawing = std::fs::read(format!("{EXAMPLES}/drawing.svg")).unwrap();
        assert_eq!(detect_bytes(&drawing), Kind::Svg);
        assert_eq!(
            detect_bytes(b"<?xml version=\"1.0\"?>\n<svg></svg>"),
            Kind::Svg
        );
        assert_eq!(detect_bytes(b"<?xml version=\"1.0\"?>\n<doc/>"), Kind::Text);
    }

    #[test]
    fn plain_text_and_code_are_not_sniffed_as_structured() {
        assert_eq!(detect_bytes(b"x = 1\nprint(x)\n"), Kind::Text);
        assert_eq!(detect_bytes(b"[um link](http://a)\n"), Kind::Text);
        assert_eq!(detect_bytes(b"[ok] feito\n"), Kind::Text);
        let yaml = std::fs::read(format!("{EXAMPLES}/config.yaml")).unwrap();
        assert_eq!(detect_bytes(&yaml), Kind::Text);
        assert_eq!(detect_bytes(&example_bytes_text("code.py")), Kind::Text);
    }

    fn example_bytes_text(name: &str) -> Vec<u8> {
        std::fs::read(format!("{EXAMPLES}/{name}")).unwrap()
    }

    #[test]
    fn bytes_inspect_only_the_first_8_kib() {
        let mut bytes = vec![b'a'; SNIFF_LEN];
        bytes.push(0);
        assert_eq!(detect_bytes(&bytes), Kind::Text);
    }

    #[test]
    fn content_type_maps_to_kind() {
        let kind = from_content_type;
        assert_eq!(kind("text/html; charset=utf-8"), Some(Kind::Html));
        assert_eq!(kind("Application/JSON"), Some(Kind::Json));
        assert_eq!(kind("application/vnd.api+json"), Some(Kind::Json));
        assert_eq!(kind("image/png"), Some(Kind::Image));
        assert_eq!(kind("image/svg+xml"), Some(Kind::Svg));
        assert_eq!(kind("application/pdf"), Some(Kind::Pdf));
        assert_eq!(kind("application/zip"), Some(Kind::Archive));
        assert_eq!(kind("video/mp4"), Some(Kind::Media));
        assert_eq!(kind("text/csv"), Some(Kind::Csv));
        assert_eq!(kind("text/markdown"), Some(Kind::Markdown));
    }

    #[test]
    fn generic_or_unknown_content_type_is_none() {
        assert_eq!(from_content_type("text/plain; charset=utf-8"), None);
        assert_eq!(from_content_type("application/octet-stream"), None);
        assert_eq!(from_content_type("application/x-algo"), None);
        assert_eq!(from_content_type("lixo"), None);
        assert_eq!(from_content_type(""), None);
    }

    #[test]
    fn stdin_dash_and_absent() {
        assert_eq!(Input::from_arg(None), Input::Stdin);
        assert_eq!(Input::from_arg(Some("-")), Input::Stdin);
        assert_eq!(detect(&Input::Stdin), Kind::Text);
    }

    #[test]
    fn names_round_trip() {
        for kind in [Kind::Markdown, Kind::Yaml, Kind::Directory, Kind::Json] {
            assert_eq!(Kind::from_name(kind.name()).unwrap(), kind);
        }
        assert!(Kind::from_name("nope").is_err());
    }
}
