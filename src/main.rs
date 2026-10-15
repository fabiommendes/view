//! `view`: roteia um arquivo, URL ou stdin para o visualizador certo.

mod backend;
mod convert;
mod detect;
mod fetch;
mod header;
mod html;
mod image;
mod pager;
mod route;
mod stdin;
mod table;

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use clap::{CommandFactory, Parser};

use crate::backend::{Backend, Internal};
use crate::convert::Format;
use crate::detect::{Input, Kind};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Mostra um arquivo, URL ou stdin com o visualizador certo"
)]
struct Cli {
    /// Arquivo, diretório ou URL. Ausente ou `-` lê o stdin.
    target: Option<String>,

    /// Imprime o cabeçalho (comando equivalente, notas e régua).
    #[arg(short, long)]
    verbose: bool,

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
    let cli = Cli::parse();
    // O `-n` com `--as` não lê o stdin, então não há o que mostrar sem alvo.
    let reads_stdin = !(cli.dry_run && cli.format.is_some());
    if reads_stdin && missing_target(cli.target.as_deref(), std::io::stdin().is_terminal()) {
        eprintln!("view: nada para mostrar (stdin é um terminal)");
        eprint!("{}", Cli::command().render_usage());
        eprintln!();
        return ExitCode::from(2);
    }
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("view: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    let target = Input::from_arg(cli.target.as_deref());
    // O `-n` com `--as` não precisa do conteúdo, então não lê nem baixa nada.
    let needs_content = !(cli.dry_run && cli.format.is_some());
    let url = cli.target.as_deref().filter(|t| detect::is_url(t));
    // Sem `--as`, só a resposta diz o tipo, e o `-n` não faz requisição.
    if let (Some(url), true, None) = (url, cli.dry_run, &cli.format) {
        println!("{}", header::from_url_unresolved(url));
        return Ok(());
    }
    // O conteúdo em memória (stdin ou URL baixada) é entregue aos backends
    // como `Input::Stdin`.
    let download = url
        .filter(|_| !cli.dry_run)
        .map(fetch::download)
        .transpose()?;
    let input = if url.is_some() { Input::Stdin } else { target };
    let (content, content_type) = match download {
        Some(download) => (Some(download.bytes), download.content_type),
        None => (
            (input == Input::Stdin && needs_content)
                .then(stdin::read_all)
                .transpose()?,
            None,
        ),
    };
    // Formato da entrada, se conhecido (sem conteúdo, o `-n` no stdin não sabe).
    let source = match &content {
        Some(bytes) => Some(
            content_type
                .as_deref()
                .and_then(detect::from_content_type)
                .unwrap_or_else(|| detect::detect_bytes(bytes)),
        ),
        None if input == Input::Stdin => None,
        None => Some(detect::detect(&input)),
    };
    let kind = match &cli.format {
        Some(name) => Kind::from_name(name)?,
        None => source.unwrap_or(Kind::Text),
    };
    let source = if Format::from_kind(kind).is_some() && source.is_some_and(is_plain_text) {
        sniff_source(&input, content.as_deref()).or(source)
    } else {
        source
    };
    let temp_path = stdin::temp_path();
    let width = header::terminal_width();
    let chain = route::chain(kind, source, width);
    let path = std::env::var_os("PATH").unwrap_or_default();
    let resolution = backend::resolve(&chain, &path)
        .ok_or_else(|| anyhow!("nenhum visualizador disponível para {}", kind.name()))?;

    let mut command = header::command_line(
        resolution.backend,
        &input,
        cli.format.is_some().then_some(kind),
        &temp_path,
    );
    if let Some(url) = url {
        command = header::from_url(url, &command);
    }
    if cli.dry_run {
        println!("{command}");
        return Ok(());
    }

    let stdout = std::io::stdout();
    let image_protocol = image::Protocol::detect(stdout.is_terminal());
    let header = if cli.verbose {
        let mut notes: Vec<String> = resolution
            .is_fallback()
            .then(|| header::install_hint(&resolution))
            .flatten()
            .into_iter()
            .collect();
        if matches!(resolution.backend, Backend::Internal(Internal::Image)) {
            notes.push(format!("# imagem: {}", image_protocol.name()));
        }
        header::render(&command, &notes, usize::from(width))
    } else {
        String::new()
    };

    let pager = pager::select(cli.no_pager, stdout.is_terminal());
    let content = Content {
        data: content.as_deref().unwrap_or_default(),
        temp_path: &temp_path,
        width,
        image_protocol,
    };
    run_backend(
        resolution.backend,
        &input,
        &content,
        &header,
        pager.as_ref(),
    )
}

/// Tipos que a detecção usa quando não reconhece o formato da entrada.
fn is_plain_text(kind: Kind) -> bool {
    matches!(kind, Kind::Text | Kind::Code)
}

/// Formato estruturado de uma entrada que a detecção não identificou (ex.
/// yaml pelo stdin, ou json num arquivo sem extensão), pelo conteúdo.
fn sniff_source(input: &Input, content: Option<&[u8]>) -> Option<Kind> {
    let bytes = match (input, content) {
        (_, Some(bytes)) => bytes.to_vec(),
        (Input::Target(path), None) => std::fs::read(path).ok()?,
        (Input::Stdin, None) => return None,
    };
    let format = convert::sniff(std::str::from_utf8(&bytes).ok()?)?;
    Some(format.kind())
}

/// Sem alvo (ou `-`) e com o stdin num terminal, ler o stdin bloqueia esperando
/// o usuário digitar, então o caso é recusado antes da leitura.
fn missing_target(target: Option<&str>, stdin_is_terminal: bool) -> bool {
    Input::from_arg(target) == Input::Stdin && stdin_is_terminal
}

/// Conteúdo em memória (stdin ou URL baixada) e onde gravá-lo para backends
/// que exigem caminho.
struct Content<'a> {
    data: &'a [u8],
    temp_path: &'a Path,
    /// Largura do terminal.
    width: u16,
    /// Protocolo usado por backends que escrevem direto no terminal.
    image_protocol: image::Protocol,
}

fn run_backend(
    backend: &Backend,
    input: &Input,
    content: &Content<'_>,
    header: &str,
    pager: &dyn pager::Pager,
) -> Result<()> {
    match backend {
        Backend::Internal(internal) if internal.writes_to_terminal() => {
            // Imagem: ignora o pager (e `--no-pager`). O cabeçalho sai antes,
            // direto no stdout, e conta nas linhas que a imagem não pode ocupar.
            let reserved = u32::try_from(header.lines().count()).unwrap_or(u32::MAX);
            let mut out = std::io::stdout().lock();
            out.write_all(header.as_bytes())?;
            return internal.render_to_terminal(
                input,
                content.data,
                content.image_protocol,
                reserved,
                &mut out,
            );
        }
        Backend::Internal(internal) => {
            let mut buffer = Vec::new();
            internal.render(input, content.data, content.width, &mut buffer)?;
            return pager.page(header, &mut buffer.as_slice());
        }
        Backend::Pipe { internal, sink } => {
            let mut buffer = Vec::new();
            internal.render(input, content.data, content.width, &mut buffer)?;
            let piped = Content {
                data: &buffer,
                temp_path: content.temp_path,
                width: content.width,
                image_protocol: content.image_protocol,
            };
            return run_backend(sink, &Input::Stdin, &piped, header, pager);
        }
        Backend::External { .. } => {}
    }
    let argv = backend
        .argv(input, content.temp_path)
        .expect("backend externo sempre tem argv");
    // Apagado ao sair da função, inclusive por erro.
    let _temp = (*input == Input::Stdin && backend.needs_path())
        .then(|| stdin::TempFile::create(content.temp_path, content.data))
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
                let _ = pipe.write_all(content.data);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_target_only_when_stdin_is_terminal() {
        assert!(missing_target(None, true));
        assert!(missing_target(Some("-"), true));
        assert!(!missing_target(None, false));
        assert!(!missing_target(Some("-"), false));
        assert!(!missing_target(Some("foto.png"), true));
    }
}
