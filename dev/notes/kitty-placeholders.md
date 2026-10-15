# Imagens com placeholders Unicode do kitty

Spike em `examples/kitty_placeholder/`. Serve para saber se imagens no `view`
podem passar por pager e pipe.

## Como funciona

- A imagem é transmitida uma vez ao terminal, direto no `/dev/tty`: PNG em
  base64, blocos de até 4096 bytes (`a=t,f=100,i=<id>,q=2,m=...`).
- Um placement virtual (`a=p,U=1,i=<id>,c=<cols>,r=<linhas>,q=2`) diz ao
  terminal que células com o placeholder `U+10EEEE` mostram pedaços da imagem.
- O stdout recebe só texto: por célula, `U+10EEEE` mais dois diacríticos (linha
  e coluna, tabela `rowcolumn-diacritics.txt` do kitty). A cor de primeiro
  plano `ESC[38;2;R;G;Bm` codifica o id de 24 bits.
- Como a imagem é texto, `less -R`, `cat` e redirecionamentos a carregam sem
  tratamento especial.
- Colunas: até 60, na proporção da imagem. A célula em pixels vem de
  `TIOCGWINSZ`; sem ela, assume 1:2.

Referência: https://sw.kovidgoyal.net/kitty/graphics-protocol/ (seção
"Unicode placeholders").

## Como testar (Ghostty 1.3.1)

```
cargo run --example kitty_placeholder -- foto.png | less -RF
cargo run --example kitty_placeholder -- foto.png | cat
```

## O que observar

- A imagem aparece inteira, sem emendas entre linhas?
- Rolar no `less` mantém a imagem, ou ela some/desloca?
- A proporção está certa (célula em pixels reportada pelo Ghostty)?
- Com `-F` e uma imagem que cabe na tela, o `less` sai e a imagem permanece?
- Depois de sair do `less`, a imagem fica no scrollback do terminal?
- Rodar duas vezes no mesmo terminal mostra imagens diferentes (ids distintos)?
- Fora do Ghostty (ou dentro de tmux), o resultado são caracteres soltos.
