//! Renderização de csv e tsv como tabela.

use anyhow::Context;
use comfy_table::{ContentArrangement, Table, presets::UTF8_FULL};

/// Linhas de dados mostradas; as demais são omitidas com um aviso.
pub const MAX_ROWS: usize = 10_000;

/// Delimitador do conteúdo: pela extensão (`.tsv` ou `.csv`) quando há e,
/// senão, pela primeira linha (tab sem vírgula vira tsv).
pub fn delimiter(name: Option<&str>, content: &[u8]) -> u8 {
    let extension = name
        .and_then(|n| n.rsplit_once('.'))
        .map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("tsv") => b'\t',
        Some("csv") => b',',
        _ => {
            let first_line = content.split(|&b| b == b'\n').next().unwrap_or_default();
            if first_line.contains(&b'\t') && !first_line.contains(&b',') {
                b'\t'
            } else {
                b','
            }
        }
    }
}

/// Texto da tabela de `content`, com a primeira linha como cabeçalho e
/// `width` colunas de largura. Mostra até `MAX_ROWS` linhas de dados e, se
/// houver mais, termina com uma linha de aviso.
pub fn render(content: &[u8], delimiter: u8, width: u16) -> anyhow::Result<String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(content);
    let mut records = reader.byte_records();
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(width);
    let cells = |record: &csv::ByteRecord| -> Vec<String> {
        record
            .iter()
            .map(|f| String::from_utf8_lossy(f).into_owned())
            .collect()
    };
    let Some(header) = records.next() else {
        return Ok(String::new());
    };
    table.set_header(cells(&header.context("csv inválido")?));
    let mut shown = 0;
    for record in records.by_ref().take(MAX_ROWS) {
        table.add_row(cells(&record.context("csv inválido")?));
        shown += 1;
    }
    let omitted = records.count();
    let mut out = format!("{table}\n");
    if omitted > 0 {
        out.push_str(&format!(
            "… {shown} linhas mostradas, {omitted} omitidas (limite de {MAX_ROWS})\n"
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delimiter_follows_extension_then_first_line() {
        assert_eq!(delimiter(Some("a.tsv"), b"a,b"), b'\t');
        assert_eq!(delimiter(Some("A.CSV"), b"a\tb"), b',');
        assert_eq!(delimiter(None, b"a\tb\n1,2"), b'\t');
        assert_eq!(delimiter(None, b"a,b\tc"), b',');
        assert_eq!(delimiter(None, b""), b',');
    }

    #[test]
    fn empty_content_renders_nothing() {
        assert_eq!(render(b"", b',', 80).unwrap(), "");
    }

    #[test]
    fn ragged_rows_are_accepted() {
        let out = render(b"a,b\n1\n1,2,3\n", b',', 80).unwrap();
        assert!(out.contains('3'));
    }
}
