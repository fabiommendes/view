---
type: issue
status: todo
tags: [csv]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
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
