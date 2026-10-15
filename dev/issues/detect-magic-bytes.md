---
type: issue
status: in_progress
tags: [detect]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Detecção de formato pelo conteúdo

Hoje `detect` usa só a extensão, então `view /usr/bin/ls` cai em texto e o
`bat` recusa o arquivo.

- R1. Ordem: diretório e prefixo `http(s)://`, depois magic bytes (`infer`),
  depois extensão (`mime_guess` mais a tabela atual), depois texto ou binário
  pelo conteúdo (bytes nulos ou UTF-8 inválido nos primeiros 8 KiB).
- R2. `--as` continua tendo prioridade sobre tudo.
- R3. Arquivo sem extensão com conteúdo de texto vira `Text`; com conteúdo
  binário vira `Binary`.

## Critérios de aceitação

- `view -n /usr/bin/ls` mostra `hexyl` ou `xxd`.
- `view -n` num PNG renomeado para `.txt` detecta imagem.
- Testes unitários cobrem cada passo da ordem.
