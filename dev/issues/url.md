---
type: issue
status: todo
tags: [url, http]
relatedTo: [dev/rfc/mvp.md, dev/issues/html.md, dev/issues/detect-magic-bytes.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# URLs http e https

- R1. Baixar com `ureq` (rustls), limite de 20 MiB, timeout de 15 s, seguindo
  redirecionamentos.
- R2. Rotear pelo `Content-Type`; se ausente ou genérico, pelo conteúdo como em
  `detect-magic-bytes`. HTML vai para a issue `html`.
- R3. Cabeçalho mostra o equivalente com `curl -sL <url> | ...`.
- R4. Erros HTTP e de rede viram mensagem curta com o status, sem pânico.

## Critérios de aceitação

- Teste com servidor HTTP local (sem rede externa) para html, json e imagem.
- `view -n https://example.com` não faz requisição.
