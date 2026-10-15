//! Download de URLs http e https para a memória.

use std::time::Duration;

use anyhow::{Result, anyhow};

/// Tamanho máximo do corpo baixado.
const MAX_BYTES: u64 = 20 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: u32 = 10;

/// Resposta baixada: o corpo e o tipo declarado pelo servidor.
#[derive(Debug)]
pub struct Download {
    pub bytes: Vec<u8>,
    /// Valor do cabeçalho `Content-Type`, se houver.
    pub content_type: Option<String>,
}

/// Baixa `url`, seguindo redirecionamentos. Os erros de rede e de status HTTP
/// viram uma mensagem curta com a URL.
pub fn download(url: &str) -> Result<Download> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .max_redirects(MAX_REDIRECTS)
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| describe(url, &e))?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .map_err(|e| describe(url, &e))?;
    Ok(Download {
        bytes,
        content_type,
    })
}

fn describe(url: &str, error: &ureq::Error) -> anyhow::Error {
    let reason = match error {
        ureq::Error::StatusCode(code) => format!("HTTP {code}"),
        ureq::Error::Timeout(_) => crate::i18n::t("error-fetch-timeout"),
        ureq::Error::BodyExceedsLimit(_) => crate::i18n::t_args(
            "error-fetch-too-large",
            &[("mib", &(MAX_BYTES / (1024 * 1024)).to_string())],
        ),
        other => other.to_string(),
    };
    anyhow!("{url}: {reason}")
}
