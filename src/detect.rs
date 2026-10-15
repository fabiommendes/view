//! Detecção do tipo de conteúdo a partir do alvo.

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
    Url,
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
            Self::Url => "url",
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
            "url" => Self::Url,
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

/// Detecta o tipo: diretório, URL e extensão, nessa ordem.
///
/// TODO: magic bytes para arquivos sem extensão ou com extensão enganosa.
pub fn detect(input: &Input) -> Kind {
    match input {
        Input::Stdin => Kind::Text,
        Input::Target(target) => {
            if is_url(target) {
                Kind::Url
            } else if Path::new(target).is_dir() {
                Kind::Directory
            } else {
                from_extension(target)
            }
        }
    }
}

pub fn is_url(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

/// Tipo a partir da extensão; sem extensão conhecida, `Text`.
pub fn from_extension(path: &str) -> Kind {
    let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) else {
        return Kind::Text;
    };
    match ext.to_ascii_lowercase().as_str() {
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
        _ => Kind::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_maps_to_kind() {
        assert_eq!(from_extension("a/b/README.md"), Kind::Markdown);
        assert_eq!(from_extension("main.RS"), Kind::Code);
        assert_eq!(from_extension("x.tar"), Kind::Archive);
        assert_eq!(from_extension("foto.jpeg"), Kind::Image);
        assert_eq!(from_extension("data.yml"), Kind::Yaml);
        assert_eq!(from_extension("notes.txt"), Kind::Text);
    }

    #[test]
    fn unknown_or_missing_extension_is_text() {
        assert_eq!(from_extension("Makefile"), Kind::Text);
        assert_eq!(from_extension("a.zzzz"), Kind::Text);
    }

    #[test]
    fn url_wins_over_extension() {
        let input = Input::Target("https://example.com/a.md".into());
        assert_eq!(detect(&input), Kind::Url);
    }

    #[test]
    fn directory_is_detected() {
        let input = Input::Target(env!("CARGO_MANIFEST_DIR").into());
        assert_eq!(detect(&input), Kind::Directory);
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
