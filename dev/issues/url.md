---
type: issue
status: completed
tags: [url, http]
relatedTo: [dev/rfc/mvp.md, dev/issues/html.md, dev/issues/detect-magic-bytes.md]
kind: enhancement
created: 2026-10-15
creator: claude
closed: 2026-10-15
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

## Plano

- [x] Dependência `ureq` (rustls) e módulo `fetch`: download com limite de 20 MiB, timeout de 15 s, redirecionamentos e erros curtos (R1, R4).
- [x] Roteamento pelo `Content-Type` (`detect::from_content_type`), com `detect_bytes` quando ausente ou genérico (R2).
- [x] Refatoração: o conteúdo do stdin vira "conteúdo em memória" (`Content`), reaproveitado pela URL baixada.
- [x] `main` baixa a URL e entrega aos backends; cabeçalho `curl -sL <url> | ...`; `-n` não faz requisição (R2, R3).
- [x] Testes de integração com servidor HTTP local (html, json, imagem, 404) e `-n`.
- [ ] `README.md` atualizado.
