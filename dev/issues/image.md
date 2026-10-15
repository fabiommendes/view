---
type: issue
status: todo
tags: [image, kitty]
relatedTo: [dev/rfc/mvp.md, dev/notes/kitty-placeholders.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Imagens direto no terminal, sem pager

Decisão do humano em 2026-10-15: no MVP, imagens não passam pelo pager. Uma
imagem raramente precisa de scroll, e o spike mostrou que os placeholders do
kitty não aparecem dentro do `less` no Ghostty (só lixo), mesmo com
`LESSUTFCHARDEF=10EEEE:p`. Com `cat` eles funcionam. Isso diverge do RFC
`mvp`, que previa placeholders dentro do pager.

- R1. Com suporte ao protocolo gráfico do kitty (`TERM=xterm-kitty`,
  `KITTY_WINDOW_ID` ou `TERM_PROGRAM=ghostty`), transmitir e exibir a imagem
  direto no stdout com `a=T` (sem placeholders e sem placement virtual).
- R2. Sem suporte, fallback em meio-bloco `▀` truecolor, também direto no
  stdout.
- R3. O backend de imagem ignora o pager. `--no-pager` não muda nada. Com
  stdout que não é TTY, só o meio-bloco faz sentido; escolha o meio-bloco.
- R4. Tamanho: no máximo a largura do terminal e a altura da tela menos as
  linhas do cabeçalho, mantendo a proporção pelo tamanho da célula em pixels
  (`TIOCGWINSZ`). Imagem menor que isso não é ampliada.
- R5. O cabeçalho sai antes da imagem: `# imagem: kitty` ou
  `# imagem: meio-bloco`. Se `chafa` estiver instalado, a dica pode citá-lo.
- R6. Reaproveitar do spike o que servir (base64, transmissão em blocos,
  tamanho da célula). Remover `examples/kitty_placeholder/` e
  `dev/notes/kitty-placeholders.md`, ou reduzir a nota a um registro curto de
  por que os placeholders não funcionaram no `less`.

## Critérios de aceitação

- Teste do meio-bloco sobre uma imagem 2x4 conhecida.
- Teste da sequência kitty gerada (cabeçalho `a=T`, blocos de até 4096 bytes,
  `m=1` em todos menos o último).
- Confirmação visual do humano no Ghostty com `tests/examples/image.png` e
  `photo.jpg`.
