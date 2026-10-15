//! Paginação. Atrás de um trait para trocar por um pager embutido depois.

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub trait Pager {
    /// Mostra `header` seguido de tudo o que `body` produzir.
    fn page(&self, header: &str, body: &mut dyn Read) -> Result<()>;
}

/// Escreve direto no stdout, sem pager.
pub struct Direct;

impl Pager for Direct {
    fn page(&self, header: &str, body: &mut dyn Read) -> Result<()> {
        let mut out = io::stdout().lock();
        copy_all(header, body, &mut out)
    }
}

/// Spawna um programa de paginação e alimenta o stdin dele.
pub struct External {
    argv: Vec<String>,
}

impl Pager for External {
    fn page(&self, header: &str, body: &mut dyn Read) -> Result<()> {
        let (program, args) = self.argv.split_first().context("pager vazio")?;
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .spawn()
            .with_context(|| format!("não foi possível executar o pager `{program}`"))?;
        let mut stdin = child.stdin.take().context("pager sem stdin")?;
        let result = copy_all(header, body, &mut stdin);
        drop(stdin);
        child.wait()?;
        result
    }
}

/// Copia, tratando pipe quebrado (usuário saiu do pager) como fim normal.
fn copy_all(header: &str, body: &mut dyn Read, out: &mut dyn Write) -> Result<()> {
    let result = out
        .write_all(header.as_bytes())
        .and_then(|()| io::copy(body, out).map(|_| ()))
        .and_then(|()| out.flush());
    match result {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => bail!(e),
        _ => Ok(()),
    }
}

/// Comando do pager: `$VIEW_PAGER`, depois `$PAGER`, padrão `less -RF`.
pub fn pager_command(view_pager: Option<&str>, pager: Option<&str>) -> Vec<String> {
    [view_pager, pager]
        .into_iter()
        .flatten()
        .map(|cmd| {
            cmd.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .find(|argv| !argv.is_empty())
        .unwrap_or_else(|| vec!["less".to_owned(), "-RF".to_owned()])
}

/// Escolhe o pager: direto se `no_pager` ou stdout não for terminal.
pub fn select(no_pager: bool, stdout_is_tty: bool) -> Box<dyn Pager> {
    if no_pager || !stdout_is_tty {
        return Box::new(Direct);
    }
    let view_pager = std::env::var("VIEW_PAGER").ok();
    let pager = std::env::var("PAGER").ok();
    Box::new(External {
        argv: pager_command(view_pager.as_deref(), pager.as_deref()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_less() {
        assert_eq!(pager_command(None, None), ["less", "-RF"]);
        assert_eq!(pager_command(Some("  "), None), ["less", "-RF"]);
    }

    #[test]
    fn view_pager_beats_pager() {
        assert_eq!(pager_command(Some("most -s"), Some("more")), ["most", "-s"]);
        assert_eq!(pager_command(None, Some("more")), ["more"]);
    }

    #[test]
    fn copy_all_writes_header_then_body() {
        let mut out = Vec::new();
        copy_all("h\n", &mut "corpo".as_bytes(), &mut out).unwrap();
        assert_eq!(out, b"h\ncorpo");
    }
}
