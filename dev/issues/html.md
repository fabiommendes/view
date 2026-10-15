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
