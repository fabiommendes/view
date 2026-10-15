//! Backends de visualização e resolução de cadeias.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::detect::Input;

/// Renderizadores escritos em Rust, ainda não implementados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Internal {
    /// Imagem via placeholders Unicode do kitty (ver `examples/kitty_placeholder.rs`).
    Image,
    Svg,
    Html,
    Csv,
    Url,
    Pdf,
    Archive,
    Media,
}

impl Internal {
    /// Enquanto for `false`, a resolução pula o backend.
    pub const fn is_implemented(self) -> bool {
        false
    }

    /// Descrição curta para o cabeçalho.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Image => "imagem interna via placeholders kitty",
            Self::Svg => "svg rasterizado internamente",
            Self::Html => "html→texto interno",
            Self::Csv => "csv→tabela interna",
            Self::Url => "download interno e roteamento pelo tipo da resposta",
            Self::Pdf => "pdf→texto interno",
            Self::Archive => "listagem interna do arquivo",
            Self::Media => "metadados internos da mídia",
        }
    }

    /// Escreve a renderização de `input` em `out`.
    pub fn render(self, _input: &Input, _out: &mut dyn std::io::Write) -> anyhow::Result<()> {
        todo!("renderizador interno {self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    /// Programa no PATH; o alvo é acrescentado ao fim dos argumentos.
    External {
        program: &'static str,
        args: Vec<String>,
        install_hint: Option<&'static str>,
        /// Não lê stdin: recebe sempre um caminho (arquivo temporário, se a
        /// entrada vier do stdin).
        needs_path: bool,
    },
    Internal(Internal),
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
            install_hint,
            needs_path: false,
        }
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
        }
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

    /// Programa e argumentos a executar; `None` para backend interno.
    ///
    /// Para stdin, o último argumento é `-` ou, se o backend exige caminho,
    /// `stdin_path` (o arquivo temporário com o conteúdo).
    pub fn argv(&self, input: &Input, stdin_path: &Path) -> Option<Vec<String>> {
        let Self::External {
            program,
            args,
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
        let chain = vec![Backend::Internal(Internal::Image), chain().remove(2)];
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
        assert!(Backend::Internal(Internal::Pdf).argv(&file, tmp).is_none());
    }
}
