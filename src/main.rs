//! `view`: roteia um arquivo, URL ou stdin para o visualizador certo.

mod backend;
mod detect;
mod header;
mod pager;
mod route;
mod stdin;

use std::io::{IsTerminal, Write};
use std::path::Path;
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
    // O `-n` com `--as` não precisa do conteúdo, então não bloqueia no stdin.
    let stdin_data = (input == Input::Stdin && !(cli.dry_run && cli.format.is_some()))
        .then(stdin::read_all)
        .transpose()?;
    let kind = match (&cli.format, &stdin_data) {
        (Some(name), _) => Kind::from_name(name)?,
        (None, Some(bytes)) => detect::detect_bytes(bytes),
        (None, None) => detect::detect(&input),
    };
    let stdin_path = stdin::temp_path();
    let width = header::terminal_width();
    let chain = route::chain(kind, width);
    let path = std::env::var_os("PATH").unwrap_or_default();
    let resolution = backend::resolve(&chain, &path)
        .ok_or_else(|| anyhow!("nenhum visualizador disponível para {}", kind.name()))?;

    let command = header::command_line(resolution.backend, &input, kind, &stdin_path);
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
    let stdin = StdinSource {
        data: stdin_data.as_deref().unwrap_or_default(),
        temp_path: &stdin_path,
    };
    run_backend(resolution.backend, &input, &stdin, &header, pager.as_ref())
}

/// Conteúdo lido do stdin e onde gravá-lo para backends que exigem caminho.
struct StdinSource<'a> {
    data: &'a [u8],
    temp_path: &'a Path,
}

fn run_backend(
    backend: &Backend,
    input: &Input,
    stdin: &StdinSource<'_>,
    header: &str,
    pager: &dyn pager::Pager,
) -> Result<()> {
    let Some(argv) = backend.argv(input, stdin.temp_path) else {
        let Backend::Internal(internal) = backend else {
            unreachable!("argv só é None para backend interno");
        };
        let mut buffer = Vec::new();
        internal.render(input, &mut buffer)?;
        return pager.page(header, &mut buffer.as_slice());
    };
    // Apagado ao sair da função, inclusive por erro.
    let _temp = (*input == Input::Stdin && backend.needs_path())
        .then(|| stdin::TempFile::create(stdin.temp_path, stdin.data))
        .transpose()?;
    let pipe_stdin = *input == Input::Stdin && !backend.needs_path();
    let (program, args) = argv.split_first().expect("argv nunca é vazio");
    let mut child = Command::new(program)
        .args(args)
        .stdin(if pipe_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("não foi possível executar `{program}`"))?;
    let mut body = child.stdout.take().context("backend sem stdout")?;
    // Escreve em outra thread: o backend pode encher o pipe de saída antes
    // de terminar de ler a entrada.
    std::thread::scope(|scope| {
        if let Some(mut pipe) = child.stdin.take() {
            scope.spawn(move || {
                // Pipe quebrado: o backend não quis ler tudo.
                let _ = pipe.write_all(stdin.data);
            });
        }
        pager.page(header, &mut body)?;
        drop(body);
        let status = child.wait()?;
        if !status.success() && !is_broken_pipe(&status) {
            bail!("`{program}` terminou com {status}");
        }
        Ok(())
    })
}

/// O backend morre com SIGPIPE quando o usuário fecha o pager antes do fim.
fn is_broken_pipe(status: &std::process::ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    status.signal() == Some(13)
}
