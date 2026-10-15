---
type: issue
status: completed
tags: [csv]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
closed: 2026-10-15
---

# CSV e TSV como tabela

- R1. Ler com `csv` (delimitador `,` para `.csv`, tab para `.tsv`) e renderizar
  com `comfy-table`, na largura do terminal.
- R2. Primeira linha é o cabeçalho da tabela.
- R3. Acima de 10 000 linhas, mostrar as primeiras 10 000 e uma linha avisando
  o corte.
- R4. Cabeçalho: `# csv→tabela interno`. Se o `csvlens` ou o `tv` estiverem
  instalados, eles não entram na cadeia agora (são interativos).

## Critérios de aceitação

- Testes com fixtures `.csv` e `.tsv`, incluindo campo com vírgula entre aspas.

## Plano

- [x] Renderizador `Internal::Csv` com `csv` e `comfy-table`: delimitador, cabeçalho e largura do terminal (R1, R2)
- [x] Corte em 10 000 linhas com aviso (R3)
- [x] Ligar à cadeia e ao cabeçalho `# csv→tabela interno`, para arquivo e conteúdo em memória (R4)
- [x] Testes com `table.csv`, `table.tsv` e o corte
- [x] `README.md` e issue em `in_review`
