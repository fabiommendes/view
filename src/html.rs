//! Conversão de HTML para markdown.

use anyhow::{Context, Result};
use htmd::HtmlToMarkdown;

/// Converte `html` em markdown. Cabeçalho, scripts, estilos e comentários
/// não aparecem na saída.
pub fn to_markdown(html: &str) -> Result<String> {
    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["head", "script", "style", "noscript", "template"])
        .build();
    let mut markdown = converter
        .convert(html)
        .with_context(|| crate::i18n::t("error-html-convert"))?;
    markdown.push('\n');
    Ok(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../tests/examples/page.html");

    #[test]
    fn page_keeps_heading_list_link_and_table() {
        let md = to_markdown(PAGE).unwrap();
        assert!(md.contains("# Bem-vindo"), "{md}");
        assert!(md.contains("*   Primeiro item"), "{md}");
        assert!(
            md.contains("[guia completo](https://example.com/docs)"),
            "{md}"
        );
        assert!(
            md.contains("Nome") && md.contains("alfa") && md.contains('|'),
            "{md}"
        );
    }

    #[test]
    fn scripts_styles_and_comments_are_dropped() {
        let md = to_markdown(PAGE).unwrap();
        assert!(!md.contains("console.log"), "{md}");
        assert!(!md.contains("font-family"), "{md}");
        let md =
            to_markdown("<p>a</p><!-- segredo --><script>x()</script><style>p{}</style>").unwrap();
        assert!(md.contains('a'));
        assert!(
            !md.contains("segredo") && !md.contains("x()") && !md.contains("p{}"),
            "{md}"
        );
    }
}
