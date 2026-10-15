---
type: issue
status: todo
tags: [pdf]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# PDF como texto

- R1. Cadeia: `pdftotext -layout <alvo> -` (poppler). Lê stdin com `-`.
- R2. Os argumentos depois do alvo (o `-` da saída) exigem que o backend
  externo aceite argumentos finais.
- R3. Sem nenhum backend disponível, o erro cita a dica de instalação
  (`pacman -S poppler`). Vale para toda cadeia, não só pdf.

## Critérios de aceitação

- Teste com `tests/examples/doc.pdf` por arquivo e por stdin.
- Teste do `-n` mostrando `pdftotext -layout <alvo> -`.

## Plano

- [ ] Argumentos finais em `Backend::External` (R2)
- [ ] Cadeia de pdf com `pdftotext` (R1)
- [ ] Dica de instalação no erro de cadeia vazia (R3)
- [ ] Testes, `README.md` e issue em `in_review`
