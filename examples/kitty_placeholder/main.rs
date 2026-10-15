//! Spike: mostra uma imagem com placeholders Unicode do protocolo gráfico do kitty.
//!
//! A transmissão da imagem vai para `/dev/tty`; as linhas de placeholder vão
//! para o stdout, então a saída pode passar por `less -RF` ou `cat`.
//! Veja `dev/notes/kitty-placeholders.md`.

mod diacritics;

use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use image::ImageFormat;

use diacritics::DIACRITICS;

const PLACEHOLDER: char = '\u{10EEEE}';
const CHUNK: usize = 4096;
const MAX_COLS: u32 = 60;
/// Lado maior máximo, em pixels, da imagem transmitida.
const MAX_PIXELS: u32 = 1600;

/// Tamanho do terminal consultado em `/dev/tty`.
struct TtySize {
    cols: u32,
    rows: u32,
    /// Largura e altura da célula em pixels, quando o terminal informa.
    cell_px: Option<(u32, u32)>,
}

fn tty_size(tty: &File) -> Option<TtySize> {
    let mut ws = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: `ws` é uma `winsize` válida e vive durante a chamada.
    let rc = unsafe { libc::ioctl(tty.as_raw_fd(), libc::TIOCGWINSZ, &mut ws) };
    if rc != 0 || ws.ws_col == 0 || ws.ws_row == 0 {
        return None;
    }
    let (cols, rows) = (u32::from(ws.ws_col), u32::from(ws.ws_row));
    let cell_px = (ws.ws_xpixel > 0 && ws.ws_ypixel > 0).then(|| {
        (
            u32::from(ws.ws_xpixel) / cols,
            u32::from(ws.ws_ypixel) / rows,
        )
    });
    Some(TtySize {
        cols,
        rows,
        cell_px,
    })
}

/// Colunas e linhas de células que mantêm a proporção da imagem.
fn grid(img_w: u32, img_h: u32, size: Option<&TtySize>) -> (u32, u32) {
    // Sem tamanho de célula em pixels, assume célula 1:2 (largura:altura).
    let (cw, ch) = size.and_then(|s| s.cell_px).unwrap_or((1, 2));
    let max_cols = size.map_or(MAX_COLS, |s| s.cols.min(MAX_COLS));
    let max_rows = size.map_or(30, |s| s.rows.saturating_sub(2).max(1));
    let max_rows = max_rows.min(DIACRITICS.len() as u32);

    let rows_for = |cols: u32| {
        let r =
            f64::from(cols) * f64::from(img_h) * f64::from(cw) / (f64::from(img_w) * f64::from(ch));
        (r.round() as u32).max(1)
    };
    let mut cols = max_cols;
    let mut rows = rows_for(cols);
    if rows > max_rows {
        rows = max_rows;
        let c =
            f64::from(rows) * f64::from(img_w) * f64::from(ch) / (f64::from(img_h) * f64::from(cw));
        cols = (c.round() as u32).clamp(1, max_cols);
    }
    (cols, rows)
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

/// Id de 24 bits, não nulo, que cabe na cor de primeiro plano.
fn random_id() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let id = (nanos ^ std::process::id().rotate_left(16)).wrapping_mul(2_654_435_761) >> 8;
    id.max(1)
}

/// Transmite a imagem (PNG) em blocos de até 4096 bytes e cria o placement virtual.
fn transmit(tty: &mut File, png: &[u8], id: u32, cols: u32, rows: u32) -> Result<()> {
    let encoded = base64(png);
    let chunks: Vec<&[u8]> = encoded.as_bytes().chunks(CHUNK).collect();
    let last = chunks.len() - 1;
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i != last);
        let controls = if i == 0 {
            format!("a=t,f=100,i={id},q=2,m={more}")
        } else {
            format!("m={more}")
        };
        write!(tty, "\x1b_G{controls};")?;
        tty.write_all(chunk)?;
        write!(tty, "\x1b\\")?;
    }
    write!(tty, "\x1b_Ga=p,U=1,i={id},c={cols},r={rows},q=2\x1b\\")?;
    tty.flush()?;
    Ok(())
}

/// Linhas de placeholder: cor de primeiro plano com o id, linha e coluna por diacríticos.
fn placeholder_lines(id: u32, cols: u32, rows: u32) -> String {
    let (r, g, b) = (id >> 16 & 0xFF, id >> 8 & 0xFF, id & 0xFF);
    let mut out = String::new();
    for row_mark in DIACRITICS.iter().take(rows as usize) {
        out.push_str(&format!("\x1b[38;2;{r};{g};{b}m"));
        for col_mark in DIACRITICS.iter().take(cols as usize) {
            out.push(PLACEHOLDER);
            out.push(*row_mark);
            out.push(*col_mark);
        }
        out.push_str("\x1b[39m\n");
    }
    out
}

fn main() -> Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        bail!("uso: kitty_placeholder <imagem>");
    };
    let mut tty = File::options()
        .write(true)
        .open("/dev/tty")
        .context("não foi possível abrir /dev/tty")?;
    let size = tty_size(&tty);

    let mut img = image::open(&path).with_context(|| format!("lendo {path}"))?;
    if img.width().max(img.height()) > MAX_PIXELS {
        img = img.thumbnail(MAX_PIXELS, MAX_PIXELS);
    }
    let (cols, rows) = grid(img.width(), img.height(), size.as_ref());
    let cols = cols.min(DIACRITICS.len() as u32);

    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)?;

    let id = random_id();
    transmit(&mut tty, &png, id, cols, rows)?;
    print!("{}", placeholder_lines(id, cols, rows));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn grid_keeps_aspect_with_default_cell() {
        // 200x200 px, célula 1:2: 60 colunas pedem 30 linhas.
        assert_eq!(grid(200, 200, None), (60, 30));
        assert_eq!(grid(400, 100, None), (60, 8));
    }

    #[test]
    fn placeholder_line_shape() {
        let text = placeholder_lines(0x0A0B0C, 2, 1);
        assert!(text.starts_with("\x1b[38;2;10;11;12m"));
        assert!(text.ends_with("\x1b[39m\n"));
        assert_eq!(text.matches(PLACEHOLDER).count(), 2);
    }

    #[test]
    fn id_fits_24_bits() {
        let id = random_id();
        assert!(id >= 1 && id <= 0xFF_FFFF);
    }
}
