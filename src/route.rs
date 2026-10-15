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
/// passa a receber a entrada convertida. Json sem `raw` passa pela conversão
/// json→json, que reformata o texto; com `raw` fica como está.
pub fn chain(kind: Kind, source: Option<Kind>, width: u16, raw: bool) -> Vec<Backend> {
    let chain = chain_for(kind, width);
    let conversion = source
        .and_then(Format::from_kind)
        .zip(Format::from_kind(kind))
        .filter(|(from, to)| from != to);
    let conversion = match conversion {
        None if kind == Kind::Json && !raw => Some((Format::Json, Format::Json)),
        other => other,
    };
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
        Kind::Image => vec![Backend::Internal(Internal::Image)],
        Kind::Pdf => vec![
            Backend::external("pdftotext", &["-layout"], Some("pacman -S poppler"))
                .trailing(&["-"]),
        ],
        Kind::Archive => vec![
            Backend::external("bsdtar", &["-tvf"], Some("pacman -S libarchive")).path_only(),
            Backend::external("tar", &["-tvf"], None).path_only(),
        ],
        Kind::Media => vec![
            Backend::external("ffprobe", &["-hide_banner"], Some("pacman -S ffmpeg"))
                .path_only()
                .merge_stderr(),
        ],
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
        assert_eq!(
            programs(&chain(Kind::Code, None, 80, false)),
            ["bat", "cat"]
        );
        assert_eq!(
            programs(&chain(Kind::Markdown, None, 80, false)),
            ["glow", "bat", "cat"]
        );
        assert_eq!(
            programs(&chain(Kind::Directory, None, 80, false)),
            ["eza", "ls"]
        );
        assert_eq!(
            programs(&chain(Kind::Binary, None, 80, false)),
            ["hexyl", "xxd"]
        );
    }

    #[test]
    fn second_wave_chains() {
        assert_eq!(programs(&chain_for(Kind::Pdf, 80)), ["pdftotext"]);
        assert_eq!(programs(&chain_for(Kind::Archive, 80)), ["bsdtar", "tar"]);
        assert!(chain_for(Kind::Archive, 80).iter().all(Backend::needs_path));
        let media = chain_for(Kind::Media, 80);
        assert_eq!(programs(&media), ["ffprobe"]);
        assert!(media[0].needs_path() && media[0].merges_stderr());
    }

    #[test]
    fn structured_conversion_pipes_into_the_target_chain() {
        let chain = chain(Kind::Yaml, Some(Kind::Json), 80, false);
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
        assert_eq!(chain(Kind::Yaml, Some(Kind::Yaml), 80, false), plain);
        assert_eq!(
            chain(Kind::Json, Some(Kind::Json), 80, true),
            chain_for(Kind::Json, 80)
        );
        assert_eq!(chain(Kind::Yaml, None, 80, false), plain);
        assert_eq!(chain(Kind::Yaml, Some(Kind::Text), 80, false), plain);
        assert_eq!(
            chain(Kind::Code, Some(Kind::Json), 80, false),
            chain_for(Kind::Code, 80)
        );
    }

    #[test]
    fn html_pipes_internal_markdown_into_the_markdown_chain() {
        let chain = chain(Kind::Html, None, 80, false);
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
        let Backend::External { args, .. } = &chain(Kind::Markdown, None, 97, false)[0] else {
            panic!("esperava backend externo");
        };
        assert_eq!(args.last().unwrap(), "97");
    }

    #[test]
    fn json_is_reformatted_unless_raw() {
        let convert = Internal::Convert {
            from: Format::Json,
            to: Format::Json,
        };
        for source in [Some(Kind::Json), None] {
            let chain = chain(Kind::Json, source, 80, false);
            assert_eq!(chain.len(), 2);
            for backend in &chain {
                let Backend::Pipe { internal, .. } = backend else {
                    panic!("esperava Pipe");
                };
                assert_eq!(*internal, convert);
            }
        }
        assert_eq!(
            chain(Kind::Json, Some(Kind::Json), 80, true),
            chain_for(Kind::Json, 80)
        );
    }
}
