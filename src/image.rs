//! Imagens direto no terminal: meio-bloco `▀` truecolor ou protocolo gráfico
//! do kitty. Não passa pelo pager (veja `dev/notes/kitty-placeholders.md`).

use std::io::Write;
use std::os::fd::AsRawFd;

use anyhow::{Context, Result};
use image::imageops::{self, FilterType};
use image::{DynamicImage, RgbaImage};

/// Célula assumida (largura, altura em pixels) quando o terminal não informa:
/// só a proporção 1:2 é conhecida, o tamanho absoluto é arbitrário.
const DEFAULT_CELL: (f64, f64) = (8.0, 16.0);

/// Tamanho do terminal ligado ao stdout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TermSize {
    pub cols: u32,
    pub rows: u32,
    /// Largura e altura da célula em pixels, quando o terminal informa.
    pub cell_px: Option<(f64, f64)>,
}

impl TermSize {
    /// Consulta `TIOCGWINSZ` no stdout. `None` se não for um terminal.
    pub fn from_stdout() -> Option<Self> {
        let mut ws = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: `ws` é uma `winsize` válida e vive durante a chamada.
        let rc =
            unsafe { libc::ioctl(std::io::stdout().as_raw_fd(), libc::TIOCGWINSZ, &raw mut ws) };
        if rc != 0 || ws.ws_col == 0 || ws.ws_row == 0 {
            return None;
        }
        let (cols, rows) = (f64::from(ws.ws_col), f64::from(ws.ws_row));
        let cell_px = (ws.ws_xpixel > 0 && ws.ws_ypixel > 0).then(|| {
            (
                f64::from(ws.ws_xpixel) / cols,
                f64::from(ws.ws_ypixel) / rows,
            )
        });
        Some(Self {
            cols: u32::from(ws.ws_col),
            rows: u32::from(ws.ws_row),
            cell_px,
        })
    }
}

/// Colunas e linhas de células que mostram uma imagem de `disp_w` x `disp_h`
/// pixels de tela, sem ampliar e sem passar de `max_cols` x `max_rows`
/// (`None`: altura livre), mantendo a proporção. `cell` é o tamanho da célula
/// em pixels de tela.
fn fit(
    disp_w: f64,
    disp_h: f64,
    cell: (f64, f64),
    max_cols: u32,
    max_rows: Option<u32>,
) -> (u32, u32) {
    let mut scale = (f64::from(max_cols.max(1)) * cell.0 / disp_w).min(1.0);
    if let Some(max_rows) = max_rows {
        scale = scale.min(f64::from(max_rows.max(1)) * cell.1 / disp_h);
    }
    let cols = (disp_w * scale / cell.0).round().max(1.0) as u32;
    let rows = (disp_h * scale / cell.1).round().max(1.0) as u32;
    (cols.min(max_cols.max(1)), rows)
}

/// Meio-bloco: imagem reduzida a `cols` x `2 * rows` pixels, um `▀` por par
/// de pixels (cima na frente, baixo no fundo). Transparência sobre preto.
fn half_blocks(img: &RgbaImage) -> String {
    let rgb = |x: u32, y: u32| {
        let [r, g, b, a] = img.get_pixel(x, y).0;
        let blend = |c: u8| (u16::from(c) * u16::from(a) / 255) as u8;
        (blend(r), blend(g), blend(b))
    };
    let mut out = String::new();
    for y in (0..img.height()).step_by(2) {
        for x in 0..img.width() {
            let (r, g, b) = rgb(x, y);
            let (br, bg, bb) = if y + 1 < img.height() {
                rgb(x, y + 1)
            } else {
                (0, 0, 0)
            };
            out.push_str(&format!("\x1b[38;2;{r};{g};{b};48;2;{br};{bg};{bb}m▀"));
        }
        out.push_str("\x1b[0m\n");
    }
    out
}

/// Redimensiona `img` para o meio-bloco e gera o texto.
fn render_half_blocks(img: &DynamicImage, size: Option<TermSize>, reserved_rows: u32) -> String {
    let cell = size.and_then(|s| s.cell_px).unwrap_or(DEFAULT_CELL);
    let max_cols = size.map_or(80, |s| s.cols);
    let max_rows = size.map(|s| s.rows.saturating_sub(reserved_rows));
    // Cada pixel da imagem vira um meio-bloco: largura da célula, meia altura.
    let (w, h) = (f64::from(img.width()), f64::from(img.height()));
    let (cols, rows) = fit(w * cell.0, h * cell.1 / 2.0, cell, max_cols, max_rows);
    let scaled = if (cols, rows * 2) == (img.width(), img.height()) {
        img.to_rgba8()
    } else {
        imageops::resize(&img.to_rgba8(), cols, rows * 2, FilterType::Triangle)
    };
    half_blocks(&scaled)
}

/// Como a imagem chega ao terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    HalfBlock,
}

impl Protocol {
    /// Kitty se o terminal anuncia suporte e o stdout é um terminal; senão,
    /// meio-bloco (que também serve para pipe e arquivo).
    pub fn detect(stdout_is_tty: bool) -> Self {
        let env = |name: &str| std::env::var(name).ok();
        if stdout_is_tty
            && supports_kitty(
                env("TERM").as_deref(),
                env("KITTY_WINDOW_ID").as_deref(),
                env("TERM_PROGRAM").as_deref(),
            )
        {
            Self::Kitty
        } else {
            Self::HalfBlock
        }
    }

    /// Nome para o cabeçalho (`# imagem: <nome>`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Kitty => "kitty",
            Self::HalfBlock => "meio-bloco",
        }
    }
}

fn supports_kitty(
    term: Option<&str>,
    kitty_window_id: Option<&str>,
    program: Option<&str>,
) -> bool {
    term == Some("xterm-kitty")
        || kitty_window_id.is_some_and(|id| !id.is_empty())
        || program == Some("ghostty")
}

/// Decodifica e escreve a imagem em `out`. `reserved_rows` são as linhas
/// já usadas (cabeçalho) que a imagem não pode ocupar.
pub fn render(
    bytes: &[u8],
    protocol: Protocol,
    size: Option<TermSize>,
    reserved_rows: u32,
    out: &mut dyn Write,
) -> Result<()> {
    let img = image::load_from_memory(bytes)
        .context("não foi possível decodificar a imagem (suportados: png, jpeg, gif, webp)")?;
    match protocol {
        Protocol::HalfBlock => {
            out.write_all(render_half_blocks(&img, size, reserved_rows).as_bytes())?
        }
        Protocol::Kitty => {
            let sequence = render_kitty(&img, size, reserved_rows)?;
            out.write_all(sequence.as_bytes())?;
        }
    }
    out.flush()?;
    Ok(())
}

/// Reduz a imagem ao tamanho exibido (sem ampliar), codifica em PNG e monta a
/// sequência kitty, terminada em newline para o prompt não cair sobre a imagem.
fn render_kitty(img: &DynamicImage, size: Option<TermSize>, reserved_rows: u32) -> Result<String> {
    let cell = size.and_then(|s| s.cell_px).unwrap_or(DEFAULT_CELL);
    let max_cols = size.map_or(80, |s| s.cols);
    // Uma linha a mais: o newline final pode rolar a tela.
    let max_rows = size.map(|s| s.rows.saturating_sub(reserved_rows + 1));
    let (w, h) = (f64::from(img.width()), f64::from(img.height()));
    let (cols, rows) = fit(w, h, cell, max_cols, max_rows);
    let target_w = (f64::from(cols) * cell.0).round().min(w) as u32;
    let target_h = (f64::from(rows) * cell.1).round().min(h) as u32;
    let rgba = img.to_rgba8();
    let scaled = if (target_w, target_h) == (img.width(), img.height()) {
        rgba
    } else {
        imageops::resize(
            &rgba,
            target_w.max(1),
            target_h.max(1),
            FilterType::Triangle,
        )
    };
    let mut png = Vec::new();
    scaled.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
    Ok(format!("{}\n", kitty_sequence(&png, cols, rows)))
}

/// Bytes por bloco de base64 do protocolo.
const CHUNK: usize = 4096;

/// Transmite e exibe um PNG (`a=T`) em `cols` x `rows` células, em blocos de
/// até 4096 bytes de base64 (`m=1` em todos menos o último). `q=2` cala as
/// respostas do terminal, que cairiam no stdin.
fn kitty_sequence(png: &[u8], cols: u32, rows: u32) -> String {
    let encoded = base64(png);
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(CHUNK)
        .map(|c| std::str::from_utf8(c).expect("base64 é ASCII"))
        .collect();
    let last = chunks.len().saturating_sub(1);
    let mut out = String::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i != last);
        let controls = if i == 0 {
            format!("a=T,f=100,c={cols},r={rows},q=2,m={more}")
        } else {
            format!("m={more}")
        };
        out.push_str(&format!("\x1b_G{controls};{chunk}\x1b\\"));
    }
    out
}

fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, b)| acc | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn size(cols: u32, rows: u32, cell_px: Option<(f64, f64)>) -> Option<TermSize> {
        Some(TermSize {
            cols,
            rows,
            cell_px,
        })
    }

    #[test]
    fn half_blocks_of_a_2x4_image() {
        let colors = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 255],
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [70, 80, 90, 0],
            [1, 2, 3, 255],
        ];
        let mut img = RgbaImage::new(2, 4);
        for (i, c) in colors.iter().enumerate() {
            img.put_pixel(i as u32 % 2, i as u32 / 2, Rgba(*c));
        }
        // O pixel (0,3) tem alfa 0: vira preto.
        let expected = "\
\x1b[38;2;255;0;0;48;2;0;0;255m▀\x1b[38;2;0;255;0;48;2;255;255;255m▀\x1b[0m\n\
\x1b[38;2;10;20;30;48;2;0;0;0m▀\x1b[38;2;40;50;60;48;2;1;2;3m▀\x1b[0m\n";
        assert_eq!(half_blocks(&img), expected);
        // 2x4 cabe no terminal: sai sem redimensionar, 2 colunas x 2 linhas.
        let text = render_half_blocks(&DynamicImage::ImageRgba8(img), size(80, 24, None), 3);
        assert_eq!(text, expected);
    }

    #[test]
    fn fit_does_not_enlarge() {
        assert_eq!(fit(20.0, 20.0, (10.0, 20.0), 80, Some(24)), (2, 1));
    }

    #[test]
    fn fit_shrinks_to_width_keeping_aspect() {
        // 1000x500 px, célula 10x20: 100x25 células cabem se não houver limite.
        assert_eq!(fit(1000.0, 500.0, (10.0, 20.0), 50, None), (50, 13));
    }

    #[test]
    fn fit_shrinks_to_height() {
        // Quadrado de 1000 px: a altura (10 linhas x 20 px) manda.
        assert_eq!(fit(1000.0, 1000.0, (10.0, 20.0), 80, Some(10)), (20, 10));
    }

    #[test]
    fn half_block_resizes_to_terminal_width() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(200, 100));
        let text = render_half_blocks(&img, size(20, 50, None), 3);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].matches('▀').count(), 20);
    }

    #[test]
    fn kitty_support_detection() {
        assert!(supports_kitty(Some("xterm-kitty"), None, None));
        assert!(supports_kitty(Some("xterm-256color"), Some("3"), None));
        assert!(supports_kitty(None, None, Some("ghostty")));
        assert!(!supports_kitty(
            Some("xterm-256color"),
            Some(""),
            Some("iTerm.app")
        ));
        assert!(!supports_kitty(None, None, None));
    }

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn kitty_sequence_chunks_payload() {
        // 6000 bytes viram 8000 de base64: dois blocos de 4096 e 3904.
        let seq = kitty_sequence(&vec![7u8; 6000], 10, 5);
        let parts: Vec<&str> = seq.split("\x1b\\").filter(|p| !p.is_empty()).collect();
        assert_eq!(parts.len(), 2);
        let (head, first) = parts[0].split_once(';').unwrap();
        assert_eq!(head, "\x1b_Ga=T,f=100,c=10,r=5,q=2,m=1");
        assert_eq!(first.len(), 4096);
        let (head, second) = parts[1].split_once(';').unwrap();
        assert_eq!(head, "\x1b_Gm=0");
        assert_eq!(second.len(), 3904);
    }

    #[test]
    fn kitty_sequence_single_chunk_is_last() {
        let seq = kitty_sequence(b"abc", 1, 1);
        assert_eq!(seq, "\x1b_Ga=T,f=100,c=1,r=1,q=2,m=0;YWJj\x1b\\");
    }

    #[test]
    fn kitty_render_ends_with_newline_and_decodes_back() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(30, 30));
        let text = render_kitty(&img, size(80, 24, Some((10.0, 20.0))), 3).unwrap();
        assert!(text.starts_with("\x1b_Ga=T,f=100,c=3,r=2,q=2,m="));
        assert!(text.ends_with("\x1b\\\n"));
    }
}
