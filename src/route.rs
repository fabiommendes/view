//! Mapa `Kind` → cadeia ordenada de backends.

use crate::backend::{Backend, Internal};
use crate::convert::Format;
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
///
/// `source` é o formato real da entrada, quando conhecido. Se ele e `kind`
/// são formatos estruturados diferentes, cada backend da cadeia de `kind`
/// passa a receber a entrada convertida.
pub fn chain(kind: Kind, source: Option<Kind>, width: u16) -> Vec<Backend> {
    let chain = chain_for(kind, width);
    let conversion = source
        .and_then(Format::from_kind)
        .zip(Format::from_kind(kind))
        .filter(|(from, to)| from != to);
    let Some((from, to)) = conversion else {
        return chain;
    };
    chain
        .into_iter()
        .map(|sink| Backend::Pipe {
            internal: Internal::Convert { from, to },
            sink: Box::new(sink),
        })
        .collect()
}

fn markdown_chain(width: u16) -> Vec<Backend> {
    vec![
        Backend::external(
            "glow",
            &["-s", "dark", "-w", &width.to_string()],
            Some("pacman -S glow"),
        ),
        bat(Some("md")),
        cat(),
    ]
}

fn chain_for(kind: Kind, width: u16) -> Vec<Backend> {
    match kind {
        Kind::Code | Kind::Text => vec![bat(None), cat()],
        Kind::Markdown => markdown_chain(width),
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
        Kind::Html => markdown_chain(width)
            .into_iter()
            .map(|sink| Backend::Pipe {
                internal: Internal::Html,
                sink: Box::new(sink),
            })
            .collect(),
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
                Backend::Internal(_) | Backend::Pipe { .. } => None,
            })
            .collect()
    }

    #[test]
    fn mvp_chains() {
        assert_eq!(programs(&chain(Kind::Code, None, 80)), ["bat", "cat"]);
        assert_eq!(
            programs(&chain(Kind::Markdown, None, 80)),
            ["glow", "bat", "cat"]
        );
        assert_eq!(programs(&chain(Kind::Directory, None, 80)), ["eza", "ls"]);
        assert_eq!(programs(&chain(Kind::Binary, None, 80)), ["hexyl", "xxd"]);
    }

    #[test]
    fn structured_conversion_pipes_into_the_target_chain() {
        let chain = chain(Kind::Yaml, Some(Kind::Json), 80);
        assert_eq!(chain.len(), 2);
        let convert = Internal::Convert {
            from: Format::Json,
            to: Format::Yaml,
        };
        for backend in &chain {
            let Backend::Pipe { internal, .. } = backend else {
                panic!("esperava Pipe");
            };
            assert_eq!(*internal, convert);
        }
        let Backend::Pipe { sink, .. } = &chain[0] else {
            unreachable!()
        };
        assert_eq!(**sink, bat(Some("yaml")));
    }

    #[test]
    fn same_or_non_structured_source_does_not_convert() {
        let plain = chain_for(Kind::Yaml, 80);
        assert_eq!(chain(Kind::Yaml, Some(Kind::Yaml), 80), plain);
        assert_eq!(chain(Kind::Yaml, None, 80), plain);
        assert_eq!(chain(Kind::Yaml, Some(Kind::Text), 80), plain);
        assert_eq!(
            chain(Kind::Code, Some(Kind::Json), 80),
            chain_for(Kind::Code, 80)
        );
    }

    #[test]
    fn html_pipes_internal_markdown_into_the_markdown_chain() {
        let chain = chain(Kind::Html, None, 80);
        let markdown = chain_for(Kind::Markdown, 80);
        assert_eq!(chain.len(), markdown.len());
        for (backend, sink) in chain.iter().zip(&markdown) {
            let Backend::Pipe {
                internal,
                sink: got,
            } = backend
            else {
                panic!("esperava Pipe");
            };
            assert_eq!(*internal, Internal::Html);
            assert_eq!(**got, *sink);
        }
    }

    #[test]
    fn glow_gets_terminal_width() {
        let Backend::External { args, .. } = &chain(Kind::Markdown, None, 97)[0] else {
            panic!("esperava backend externo");
        };
        assert_eq!(args.last().unwrap(), "97");
    }
}
