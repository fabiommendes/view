# Placeholders Unicode do kitty: por que não usamos

Registro de um spike (removido; está no histórico do git, antes do commit que
fecha a issue `image`).

- A ideia: transmitir a imagem ao terminal pelo `/dev/tty` e mandar ao stdout
  só texto (`U+10EEEE` mais diacríticos de linha e coluna, cor de primeiro
  plano com o id), para a imagem passar por `less` e pipes.
- No Ghostty 1.3.1 funciona com `cat`, mas dentro do `less -RF` os placeholders
  não aparecem (só lixo), mesmo com `LESSUTFCHARDEF=10EEEE:p`.
- Decisão (2026-10-15): imagens não passam pelo pager. O `view` transmite com
  `a=T` direto no stdout (ou usa meio-bloco), em `src/image.rs`.
- Referência: https://sw.kovidgoyal.net/kitty/graphics-protocol/ (seção
  "Unicode placeholders").
