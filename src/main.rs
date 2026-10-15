//! `view`: roteia um arquivo, URL ou stdin para o visualizador certo.

mod backend;
mod detect;
mod header;
mod pager;
mod route;

use std::io::IsTerminal;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;

use crate::backend::Backend;
use crate::detect::{Input, Kind};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Mostra um arquivo, URL ou stdin com o visualizador certo"
)]
struct Cli {
    /// Arquivo, diretório ou URL. Ausente ou `-` lê o stdin.
    target: Option<String>,

    /// Não imprime o cabeçalho (comando equivalente e régua).
    #[arg(short, long)]
    quiet: bool,

    /// Só imprime o comando que rodaria.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Força o formato (md, json, yaml, code, dir, binary...).
    #[arg(long = "as", value_name = "FORMAT")]
    format: Option<String>,

    /// Escreve direto no stdout, sem pager.
    #[arg(long)]
    no_pager: bool,
}

fn main() -> ExitCode {
    match run(&Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("view: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    let input = Input::from_arg(cli.target.as_deref());
    let kind = match &cli.format {
        Some(name) => Kind::from_name(name)?,
        None => detect::detect(&input),
    };
    let width = header::terminal_width();
    let chain = route::chain(kind, width);
    let path = std::env::var_os("PATH").unwrap_or_default();
    let resolution = backend::resolve(&chain, &path)
        .ok_or_else(|| anyhow!("nenhum visualizador disponível para {}", kind.name()))?;

    let command = header::command_line(resolution.backend, &input, kind);
    if cli.dry_run {
        println!("{command}");
        return Ok(());
    }

    let header = if cli.quiet {
        String::new()
    } else {
        let hint = resolution
            .is_fallback()
            .then(|| header::install_hint(&resolution))
            .flatten();
        header::render(&command, hint.as_deref(), usize::from(width))
    };

    let stdout = std::io::stdout();
    let pager = pager::select(cli.no_pager, stdout.is_terminal());
    run_backend(resolution.backend, &input, &header, pager.as_ref())
}

fn run_backend(
    backend: &Backend,
    input: &Input,
    header: &str,
    pager: &dyn pager::Pager,
) -> Result<()> {
    let Some(argv) = backend.argv(input) else {
        let Backend::Internal(internal) = backend else {
            unreachable!("argv só é None para backend interno");
        };
        let mut buffer = Vec::new();
        internal.render(input, &mut buffer)?;
        return pager.page(header, &mut buffer.as_slice());
    };
    let (program, args) = argv.split_first().expect("argv nunca é vazio");
    let mut child = Command::new(program)
        .args(args)
        .stdin(if *input == Input::Stdin {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("não foi possível executar `{program}`"))?;
    let mut body = child.stdout.take().context("backend sem stdout")?;
    pager.page(header, &mut body)?;
    drop(body);
    let status = child.wait()?;
    if !status.success() && !is_broken_pipe(&status) {
        bail!("`{program}` terminou com {status}");
    }
    Ok(())
}

/// O backend morre com SIGPIPE quando o usuário fecha o pager antes do fim.
fn is_broken_pipe(status: &std::process::ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    status.signal() == Some(13)
}
