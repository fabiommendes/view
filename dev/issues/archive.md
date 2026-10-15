---
type: issue
status: in_review
tags: [archive]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Listagem de arquivos compactados

- R1. Cadeia: `bsdtar -tvf <alvo>` (libarchive) → `tar -tvf <alvo>`. O
  `bsdtar` lê zip, tar.*, 7z, rar e iso; o `tar` do GNU não lê zip.
- R2. Recebem sempre um caminho (arquivo temporário para stdin e URL): zip
  pede acesso aleatório para listar direito.

## Critérios de aceitação

- Testes com `tests/examples/archive.zip` e `archive.tar.gz`, por arquivo e
  por stdin.

## Plano

- [x] Cadeia `bsdtar` → `tar`, só com caminho (R1, R2)
- [x] Testes, `README.md` e issue em `in_review`
