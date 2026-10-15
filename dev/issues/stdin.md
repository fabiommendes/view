---
type: issue
status: todo
tags: [detect, cli]
relatedTo: [dev/rfc/mvp.md, dev/issues/detect-magic-bytes.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Leitura de stdin

`cmd | view` e `view -` devem detectar o formato pelo conteúdo e seguir a
cadeia normal.

- R1. Ler stdin inteiro em memória e detectar com as mesmas regras de
  `detect-magic-bytes`.
- R2. Backends externos que aceitam stdin recebem os bytes por pipe. Os que
  exigem caminho recebem um arquivo temporário, apagado ao final.
- R3. O cabeçalho mostra o comando com `-` ou com o caminho temporário,
  conforme o caso.

## Critérios de aceitação

- `cat Cargo.toml | view --no-pager -q` sai com highlight de TOML.
- `printf '# a' | view --as md -n` mostra o comando do `glow`.
