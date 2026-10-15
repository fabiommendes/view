---
type: issue
status: in_progress
tags: [html, markdown]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# HTML renderizado via markdown

- R1. Converter HTML para markdown com `htmd` e mandar o resultado para a
  cadeia de markdown (`glow` → `bat -l md` → `cat`).
- R2. Cabeçalho: `# html→md interno` seguido do comando do backend de
  markdown escolhido.
- R3. Scripts, estilos e comentários HTML não aparecem na saída.

## Critérios de aceitação

- Uma fixture HTML com título, lista, link e tabela vira markdown com os
  quatro elementos (teste unitário sobre a conversão).
- `view -n fixture.html` mostra o comando do `glow`.

## Plano

- [x] Dependência `htmd` e módulo `html` (html→markdown, sem script, estilo e comentário) com testes unitários (R1, R3).
- [x] `Internal::Html` implementado e rota `html→md interno | <cadeia de markdown>` com `Backend::Pipe` (R1, R2).
- [x] Detecção de HTML no stdin pelo conteúdo (`<!doctype html` ou `<html` no começo).
- [ ] Testes de integração (`--no-pager -q`, `-n`) sobre `page.html` e `README.md`.
