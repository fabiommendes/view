---
type: issue
status: todo
tags: [image, kitty]
relatedTo: [dev/rfc/mvp.md, dev/notes/kitty-placeholders.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Imagens com placeholders do kitty

O spike em `examples/kitty_placeholder/` funciona com `cat`. Com `less`, os
placeholders só passam com `LESSUTFCHARDEF=10EEEE:p`: sem isso o `less` trata
`U+10EEEE` (uso privado) como não imprimível e descarta a grade. Verificado
num pty; falta a confirmação visual do humano no Ghostty.

- R1. Mover o encoder do spike para `src/` (transmissão em `/dev/tty`,
  placement virtual `U=1`, grade de placeholders no stdout).
- R2. Ativar só quando `TERM=xterm-kitty`, `KITTY_WINDOW_ID` ou
  `TERM_PROGRAM=ghostty`. Fora disso, fallback em meio-bloco `▀` truecolor.
- R3. O módulo `pager` acrescenta `10EEEE:p` ao `LESSUTFCHARDEF` ao spawnar
  `less`, preservando o valor do usuário (a lista exige ordem crescente).
- R4. Ao sair do pager, apagar a imagem do terminal (`a=d,d=I,i=<id>`).
- R5. Tamanho: no máximo a largura do terminal e a altura da tela, mantendo a
  proporção pelo tamanho da célula em pixels (`TIOCGWINSZ`).
- R6. Cabeçalho: `# imagem: kitty placeholders` ou `# imagem: meio-bloco`. Se
  `chafa` estiver instalado, a dica pode citá-lo como equivalente.
- R7. Remover o exemplo do spike depois que o código estiver em `src/`.

## Critérios de aceitação

- Testes unitários da grade (diacríticos, cor do id) e do merge de
  `LESSUTFCHARDEF`.
- Teste do meio-bloco sobre uma imagem 2x4 conhecida.
- Confirmação visual do humano no Ghostty com `less`.
