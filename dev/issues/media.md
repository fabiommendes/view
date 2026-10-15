---
type: issue
status: in_review
tags: [media]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Metadados de áudio e vídeo

- R1. Cadeia: `ffprobe -hide_banner <alvo>`. O resumo do `ffprobe` sai no
  stderr, então o backend junta stderr ao stdout (o cabeçalho mostra `2>&1`).
- R2. Recebe sempre um caminho: mp4 com o índice no fim não é legível por
  pipe.

## Critérios de aceitação

- Teste com `tests/examples/sound.wav` por arquivo e por stdin.

## Plano

- [x] Backend externo que junta stderr ao stdout (R1)
- [x] Cadeia de mídia com `ffprobe`, só com caminho (R1, R2)
- [x] Testes, `README.md` e issue em `in_review`
