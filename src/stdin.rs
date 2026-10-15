//! Leitura de stdin e arquivo temporário para backends que exigem caminho.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Lê o stdin inteiro em memória.
pub fn read_all() -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .lock()
        .read_to_end(&mut bytes)
        .with_context(|| crate::i18n::t("error-stdin-read"))?;
    Ok(bytes)
}

/// Caminho que o arquivo temporário do stdin usa neste processo. É estável
/// dentro do processo, então o `-n` mostra o mesmo caminho da execução real.
pub fn temp_path() -> PathBuf {
    std::env::temp_dir().join(format!("view-stdin-{}", std::process::id()))
}

/// Arquivo temporário (modo 0600) apagado quando o guard sai de escopo,
/// inclusive em caminhos de erro.
#[derive(Debug)]
pub struct TempFile {
    path: PathBuf,
}

impl TempFile {
    /// Cria `path` com o conteúdo `bytes`. Falha se o caminho já existe, para
    /// não seguir um link simbólico plantado.
    pub fn create(path: &Path, bytes: &[u8]) -> Result<Self> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .with_context(|| {
                crate::i18n::t_args(
                    "error-create-file",
                    &[("path", &path.display().to_string())],
                )
            })?;
        // O guard já vale: se a escrita falhar, o arquivo é apagado.
        let guard = Self {
            path: path.to_owned(),
        };
        file.write_all(bytes).with_context(|| {
            crate::i18n::t_args("error-write-file", &[("path", &path.display().to_string())])
        })?;
        Ok(guard)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn unique(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("view-stdin-test-{name}-{}", std::process::id()))
    }

    #[test]
    fn creates_private_file_and_removes_on_drop() {
        let path = unique("drop");
        let guard = TempFile::create(&path, b"oi").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"oi");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        drop(guard);
        assert!(!path.exists());
    }

    #[test]
    fn refuses_existing_path() {
        let path = unique("exists");
        let _guard = TempFile::create(&path, b"a").unwrap();
        assert!(TempFile::create(&path, b"b").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"a");
    }
}
