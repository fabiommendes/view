//! Backends de visualização e resolução de cadeias.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, bail};

use crate::convert::{self, Format};
use crate::detect::Input;
use crate::html;
use crate::image;
use crate::table;

/// Renderizadores escritos em Rust. Os ainda não implementados ficam fora da
/// resolução (ver `is_implemented`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Internal {
    /// Converte entre json, yaml e toml.
    Convert {
        from: Format,
        to: Format,
    },
    /// Imagem direto no terminal (kitty ou meio-bloco), fora do pager.
    Image,
    Svg,
    Html,
    Csv,
}

impl Internal {
    /// Enquanto for `false`, a resolução pula o backend.
    pub const fn is_implemented(self) -> bool {
        matches!(
            self,
            Self::Convert { .. } | Self::Html | Self::Csv | Self::Image
        )
    }

    /// O renderizador escreve direto no terminal (stdout), sem passar pelo
    /// pager: o chamador imprime o cabeçalho e usa `render_to_terminal`.
    pub const fn writes_to_terminal(self) -> bool {
        matches!(self, Self::Image)
    }

    /// Descrição curta para o cabeçalho.
    pub fn description(self) -> String {
        let text = match self {
            Self::Convert { from, to } => {
                return format!("{}→{} interno", from.name(), to.name());
            }
            Self::Image => "imagem interna",
            Self::Svg => "svg rasterizado internamente",
            Self::Html => "html→md interno",
            Self::Csv => "csv→tabela interno",
        };
        text.to_owned()
    }

    /// Escreve a renderização de `input` em `out`. `content` é o conteúdo já
    /// em memória (stdin ou URL baixada) quando `input` é `Input::Stdin`;
    /// `width` é a largura do terminal.
    pub fn render(
        self,
        input: &Input,
        content: &[u8],
        width: u16,
        out: &mut dyn std::io::Write,
    ) -> anyhow::Result<()> {
        match self {
            Self::Convert { from, to } => {
                let bytes = source_bytes(input, content)?;
                let text = String::from_utf8(bytes).context("entrada não é UTF-8")?;
                out.write_all(convert::convert(&text, from, to)?.as_bytes())?;
                Ok(())
            }
            Self::Html => {
                let bytes = source_bytes(input, content)?;
                let text = String::from_utf8(bytes).context("entrada não é UTF-8")?;
                out.write_all(html::to_markdown(&text)?.as_bytes())?;
                Ok(())
            }
            Self::Csv => {
                let bytes = source_bytes(input, content)?;
                let name = match input {
                    Input::Target(path) => Some(path.as_str()),
                    Input::Stdin => None,
                };
                let delimiter = table::delimiter(name, &bytes);
                out.write_all(table::render(&bytes, delimiter, width)?.as_bytes())?;
                Ok(())
            }
            Self::Image => bail!("imagem escreve direto no terminal, sem pager"),
            _ => todo!("renderizador interno {self:?}"),
        }
    }

    /// Escreve a renderização de `input` direto no terminal. Só vale para
    /// `writes_to_terminal`. `reserved_rows` são as linhas já ocupadas pelo
    /// cabeçalho; `protocol` é o protocolo anunciado nele.
    pub fn render_to_terminal(
        self,
        input: &Input,
        content: &[u8],
        protocol: image::Protocol,
        reserved_rows: u32,
        out: &mut dyn std::io::Write,
    ) -> anyhow::Result<()> {
        match self {
            Self::Image => {
                let bytes = source_bytes(input, content)?;
                image::render(
                    &bytes,
                    protocol,
                    image::TermSize::from_stdout(),
                    reserved_rows,
                    out,
                )
            }
            _ => bail!("{self:?} não escreve direto no terminal"),
        }
    }
}

/// Bytes da entrada: o conteúdo em memória ou o arquivo.
fn source_bytes(input: &Input, content: &[u8]) -> anyhow::Result<Vec<u8>> {
    match input {
        Input::Stdin => Ok(content.to_vec()),
        Input::Target(path) => {
            std::fs::read(path).with_context(|| format!("não foi possível ler {path}"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    /// Programa no PATH; o alvo vem depois de `args` e antes de `trailing`.
    External {
        program: &'static str,
        args: Vec<String>,
        /// Argumentos depois do alvo (ex. o `-` de saída do `pdftotext`).
        trailing: Vec<String>,
        install_hint: Option<&'static str>,
        /// Não lê stdin: recebe sempre um caminho (arquivo temporário, se o
        /// conteúdo estiver em memória).
        needs_path: bool,
        /// O programa escreve o resultado no stderr (ex. `ffprobe`): o
        /// stderr vai para o mesmo pipe do stdout.
        merge_stderr: bool,
    },
    Internal(Internal),
    /// Renderizador interno cuja saída vira o stdin de outro backend
    /// (ex. `json→yaml` interno e depois `bat -l yaml`).
    Pipe {
        internal: Internal,
        sink: Box<Backend>,
    },
}

impl Backend {
    pub fn external(
        program: &'static str,
        args: &[&str],
        install_hint: Option<&'static str>,
    ) -> Self {
        Self::External {
            program,
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            trailing: Vec::new(),
            install_hint,
            needs_path: false,
            merge_stderr: false,
        }
    }

    /// Acrescenta argumentos depois do alvo.
    pub fn trailing(mut self, extra: &[&str]) -> Self {
        if let Self::External { trailing, .. } = &mut self {
            trailing.extend(extra.iter().map(|a| (*a).to_owned()));
        }
        self
    }

    /// Junta o stderr do programa ao stdout.
    pub fn merge_stderr(mut self) -> Self {
        if let Self::External { merge_stderr, .. } = &mut self {
            *merge_stderr = true;
        }
        self
    }

    /// Marca um backend externo que exige caminho e não lê stdin.
    pub fn path_only(mut self) -> Self {
        if let Self::External { needs_path, .. } = &mut self {
            *needs_path = true;
        }
        self
    }

    pub fn is_available(&self, path: &OsStr) -> bool {
        match self {
            Self::External { program, .. } => in_path(program, path),
            Self::Internal(internal) => internal.is_implemented(),
            Self::Pipe { internal, sink } => internal.is_implemented() && sink.is_available(path),
        }
    }

    /// O stderr do backend vai para o mesmo pipe do stdout.
    pub fn merges_stderr(&self) -> bool {
        matches!(
            self,
            Self::External {
                merge_stderr: true,
                ..
            }
        )
    }

    /// O backend exige um caminho e não lê stdin.
    pub fn needs_path(&self) -> bool {
        matches!(
            self,
            Self::External {
                needs_path: true,
                ..
            }
        )
    }

    /// Programa e argumentos a executar; `None` para backend interno ou `Pipe`.
    ///
    /// Para conteúdo em memória (`Input::Stdin`), o alvo é `-` ou, se o
    /// backend exige caminho, `stdin_path` (o arquivo temporário com o
    /// conteúdo).
    pub fn argv(&self, input: &Input, stdin_path: &Path) -> Option<Vec<String>> {
        let Self::External {
            program,
            args,
            trailing,
            needs_path,
            ..
        } = self
        else {
            return None;
        };
        let mut argv = vec![(*program).to_owned()];
        argv.extend(args.iter().cloned());
        argv.push(match input {
            Input::Target(target) => target.clone(),
            Input::Stdin if *needs_path => stdin_path.to_string_lossy().into_owned(),
            Input::Stdin => "-".to_owned(),
        });
        argv.extend(trailing.iter().cloned());
        Some(argv)
    }
}

/// Backend escolhido e os que ficaram para trás.
#[derive(Debug)]
pub struct Resolution<'a> {
    pub backend: &'a Backend,
    /// Backends preferidos que não estão disponíveis, em ordem.
    pub skipped: &'a [Backend],
}

impl Resolution<'_> {
    pub fn is_fallback(&self) -> bool {
        !self.skipped.is_empty()
    }
}

/// Primeiro backend disponível da cadeia, dado o valor de `PATH`.
pub fn resolve<'a>(chain: &'a [Backend], path: &OsStr) -> Option<Resolution<'a>> {
    let index = chain.iter().position(|b| b.is_available(path))?;
    Some(Resolution {
        backend: &chain[index],
        skipped: &chain[..index],
    })
}

fn in_path(program: &str, path: &OsStr) -> bool {
    std::env::split_paths(path).any(|dir| {
        let candidate = dir.join(program);
        is_executable(&candidate)
    })
}

fn is_executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Diretório temporário com executáveis falsos, para controlar o PATH.
    fn fake_path(name: &str, programs: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("view-test-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for program in programs {
            let file = dir.join(program);
            fs::write(&file, "#!/bin/sh\n").unwrap();
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        }
        dir
    }

    fn chain() -> Vec<Backend> {
        vec![
            Backend::external("glow", &[], Some("pacman -S glow")),
            Backend::external("bat", &[], None),
            Backend::external("cat", &[], None),
        ]
    }

    #[test]
    fn first_available_wins_without_fallback() {
        let dir = fake_path("first", &["glow", "bat", "cat"]);
        let chain = chain();
        let res = resolve(&chain, dir.as_os_str()).unwrap();
        assert!(!res.is_fallback());
        assert_eq!(res.backend, &chain[0]);
    }

    #[test]
    fn falls_back_and_reports_skipped() {
        let dir = fake_path("fallback", &["bat", "cat"]);
        let chain = chain();
        let res = resolve(&chain, dir.as_os_str()).unwrap();
        assert!(res.is_fallback());
        assert_eq!(res.backend, &chain[1]);
        assert_eq!(res.skipped, &chain[..1]);
    }

    #[test]
    fn nothing_available() {
        let dir = fake_path("none", &[]);
        assert!(resolve(&chain(), dir.as_os_str()).is_none());
    }

    #[test]
    fn non_executable_file_is_ignored() {
        let dir = fake_path("noexec", &[]);
        fs::write(dir.join("glow"), "").unwrap();
        assert!(!chain()[0].is_available(dir.as_os_str()));
    }

    #[test]
    fn unimplemented_internal_is_skipped() {
        let dir = fake_path("internal", &["cat"]);
        let chain = vec![Backend::Internal(Internal::Svg), chain().remove(2)];
        let res = resolve(&chain, dir.as_os_str()).unwrap();
        assert!(res.is_fallback());
    }

    #[test]
    fn argv_ends_with_target_dash_or_temp_path() {
        let tmp = Path::new("/tmp/x");
        let backend = Backend::external("bat", &["-l", "md"], None);
        let file = Input::Target("a.md".into());
        assert_eq!(
            backend.argv(&file, tmp).unwrap(),
            ["bat", "-l", "md", "a.md"]
        );
        assert_eq!(
            backend.argv(&Input::Stdin, tmp).unwrap(),
            ["bat", "-l", "md", "-"]
        );
        let ls = Backend::external("ls", &["-la"], None).path_only();
        assert!(ls.needs_path());
        assert_eq!(
            ls.argv(&Input::Stdin, tmp).unwrap(),
            ["ls", "-la", "/tmp/x"]
        );
        assert!(Backend::Internal(Internal::Svg).argv(&file, tmp).is_none());
    }

    #[test]
    fn trailing_args_come_after_the_target() {
        let tmp = Path::new("/tmp/x");
        let pdf = Backend::external("pdftotext", &["-layout"], None).trailing(&["-"]);
        assert_eq!(
            pdf.argv(&Input::Target("a.pdf".into()), tmp).unwrap(),
            ["pdftotext", "-layout", "a.pdf", "-"]
        );
        assert_eq!(
            pdf.argv(&Input::Stdin, tmp).unwrap(),
            ["pdftotext", "-layout", "-", "-"]
        );
    }
}
