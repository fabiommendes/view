//! Mapa `Kind` → cadeia ordenada de backends.

use crate::backend::{Backend, Internal};
use crate::detect::Kind;

const BAT_HINT: Option<&str> = Some("pacman -S bat");

fn bat(lang: Option<&str>) -> Backend {
    let mut args = vec!["--color=always", "--paging=never", "--style=plain"];
    if let Some(lang) = lang {
        args.extend(["-l", lang]);
    }
    Backend::external("bat", &args, BAT_HINT)
}

fn cat() -> Backend {
    Backend::external("cat", &[], None)
}

/// Cadeia de backends para `kind`; `width` é a largura do terminal.
pub fn chain(kind: Kind, width: u16) -> Vec<Backend> {
    match kind {
        Kind::Code | Kind::Text => vec![bat(None), cat()],
        Kind::Markdown => vec![
            Backend::external(
                "glow",
                &["-s", "dark", "-w", &width.to_string()],
                Some("pacman -S glow"),
            ),
            bat(Some("md")),
            cat(),
        ],
        Kind::Directory => vec![
            Backend::external(
                "eza",
                &["-la", "--tree", "-L2", "--color=always"],
                Some("pacman -S eza"),
            )
            .path_only(),
            Backend::external("ls", &["-la", "--color=always"], None).path_only(),
        ],
        Kind::Binary => vec![
            Backend::external("hexyl", &[], Some("pacman -S hexyl")),
            Backend::external("xxd", &[], None),
        ],
        Kind::Json => vec![bat(Some("json")), cat()],
        Kind::Yaml => vec![bat(Some("yaml")), cat()],
        Kind::Toml => vec![bat(Some("toml")), cat()],
        Kind::Html => vec![Backend::Internal(Internal::Html), bat(Some("html")), cat()],
        Kind::Svg => vec![Backend::Internal(Internal::Svg), bat(Some("xml")), cat()],
        Kind::Csv => vec![Backend::Internal(Internal::Csv), bat(Some("csv")), cat()],
        Kind::Url => vec![Backend::Internal(Internal::Url)],
        Kind::Image => vec![Backend::Internal(Internal::Image)],
        Kind::Pdf => vec![Backend::Internal(Internal::Pdf)],
        Kind::Archive => vec![Backend::Internal(Internal::Archive)],
        Kind::Media => vec![Backend::Internal(Internal::Media)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn programs(chain: &[Backend]) -> Vec<&str> {
        chain
            .iter()
            .filter_map(|b| match b {
                Backend::External { program, .. } => Some(*program),
                Backend::Internal(_) => None,
            })
            .collect()
    }

    #[test]
    fn mvp_chains() {
        assert_eq!(programs(&chain(Kind::Code, 80)), ["bat", "cat"]);
        assert_eq!(programs(&chain(Kind::Markdown, 80)), ["glow", "bat", "cat"]);
        assert_eq!(programs(&chain(Kind::Directory, 80)), ["eza", "ls"]);
        assert_eq!(programs(&chain(Kind::Binary, 80)), ["hexyl", "xxd"]);
    }

    #[test]
    fn glow_gets_terminal_width() {
        let Backend::External { args, .. } = &chain(Kind::Markdown, 97)[0] else {
            panic!("esperava backend externo");
        };
        assert_eq!(args.last().unwrap(), "97");
    }
}
