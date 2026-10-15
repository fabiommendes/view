---
type: issue
status: in_progress
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

## Plano

- [ ] Detectar o formato do stdin pelo conteúdo (magic bytes e texto ou binário), com testes unitários.
- [ ] Backends que exigem caminho (`eza`, `ls`) recebem um arquivo temporário com permissão 0600, apagado por um guard com `Drop`.
- [ ] Demais backends recebem os bytes por pipe; o comando do cabeçalho mostra `-` ou o caminho temporário.
- [ ] Testes de integração com stdin em pipe e `just check`.
