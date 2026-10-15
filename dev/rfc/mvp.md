---
type: rfc
status: accepted
tags: [mvp, pager, formats]
relatedTo: [dev/notes/kitty-placeholders.md]
created: 2026-10-15
decided: 2026-10-15
creator: claude
---

# MVP do view

## Problem

Cada formato de arquivo tem o seu visualizador de terminal (`glow`, `bat`,
`eza`, `pdftotext`...), cada um com flags próprias, e nem todos estão
instalados em toda máquina. Quem quer só olhar um arquivo precisa lembrar qual
ferramenta usar e como fazer ela paginar com cor. Alguns formatos (YAML,
CSV, HTML, imagem) não têm visualizador instalado nesta máquina.

## Proposal

### Modelo

- Cada formato (`detect::Kind`) tem uma cadeia ordenada de backends. Vence o
  primeiro disponível.
- Um backend externo é um programa no PATH, chamado com cor forçada e sem o
  pager dele. Um backend interno é código Rust que usa crates do ecossistema.
  Nenhum parser de formato escrito à mão.
- Toda saída vai para um único pager: `$VIEW_PAGER`, `$PAGER` ou `less -RF`.
  Sem TTY na saída, ou com `--no-pager`, escreve direto.
- O pager fica atrás de uma interface, para que o `minus` (pager embutido)
  possa entrar depois sem mudar o resto.

### Cabeçalho

A primeira linha paginada mostra o comando equivalente, esmaecido, e uma
régua `─` até a largura do terminal. Quando a cadeia caiu num fallback, uma
linha extra sugere instalar a ferramenta preferida. `-q` suprime as duas.
`-n` imprime só o comando, sem executar.

```
$ eza -la --tree -L2 --color=always
# glow não encontrado: pacman -S glow
────────────────────────────────────────────
```

### CLI

`view [-q] [-n] [--no-pager] [--as FORMAT] [TARGET]`. Sem `TARGET`, ou `-`,
lê stdin. `--as` força o formato e é o toggle json/yaml/toml.

### Detecção

Magic bytes (`infer`), depois extensão (`mime_guess`), depois texto ou
binário pelo conteúdo. Diretório e prefixo `http(s)://` antes de tudo.

### Cadeias

`int` marca backend interno.

| Formato | Cadeia |
| :-- | :-- |
| código e texto | `bat` → `cat` |
| markdown | `glow` → `bat -l md` → `cat` |
| html | int `htmd` (html para markdown) → cadeia de markdown |
| URL | int `ureq` baixa, roteia pelo `Content-Type` |
| json, yaml, toml | `bat`; com `--as`: int `serde` converte → `bat -l <formato>` |
| imagem | int placeholders do kitty (kitty, Ghostty) → int meio-bloco `▀` truecolor |
| csv, tsv | int `csv` + `comfy-table` |
| diretório | `eza -la --tree -L2` → `ls -la` |
| binário | `hexyl` → `xxd` |

Segunda leva, fora do MVP: svg (`resvg` → imagem), pdf (`pdftotext -layout`),
arquivos compactados (`bsdtar -tvf`), mídia (`ffprobe`), arquivo de
configuração com as cadeias.

### Crates

| Uso | Crate | Motivo |
| :-- | :-- | :-- |
| detecção | `infer`, `mime_guess` | magic bytes sem dependências; extensão |
| json, yaml, toml | `serde_json` (`preserve_order`), `serde-saphyr`, `toml` | `serde_yaml` e `serde_yml` estão abandonados |
| html | `htmd` | reaproveita a cadeia de markdown |
| http | `ureq` | bloqueante e pequeno; `reqwest` puxa tokio |
| csv | `csv`, `comfy-table` | tabela ciente de ANSI |
| imagem | `image` | decodifica e redimensiona; o encoder kitty é nosso, pequeno |

Highlight fica com o `bat` externo. `syntect` só entra se precisarmos rodar
sem `bat`.

### Issues previstas

1. `detect-magic-bytes`: detecção por conteúdo (corrige `view /usr/bin/ls`).
2. `structured-data`: `--as` entre json, yaml e toml.
3. `html`: html para markdown.
4. `url`: download e roteamento por `Content-Type`.
5. `csv`: tabela.
6. `image`: placeholders do kitty e fallback em meio-bloco. Depende do spike.
7. `stdin`: detecção de stdin pelo conteúdo, com arquivo temporário para
   backends que exigem caminho.

## Alternatives

- `bat` como lib (`PrettyPrinter`): ganha pouco sobre o `bat` externo e esconde
  o comando do usuário.
- `minus` como pager padrão: permitiria tecla de toggle json/yaml, mas é menos
  testado e não sabemos se passa os placeholders. Fica para depois.
- `mdcat` como lib para markdown com imagens: só renderiza imagem via ratatui e
  o projeto original não é mais mantido.
- `viuer` para imagens: escreve direto no stdout e não usa placeholders, então
  não funciona no pager.
- `w3m`/`lynx` para html: não estão instalados e não ganham do `htmd` + `glow`.

## Open questions

- O spike de placeholders funciona com `cat` no Ghostty 1.3.1. Com `less`, só
  com `LESSUTFCHARDEF=10EEEE:p`; detalhes na issue `image`.
- Imagens dentro de markdown e html ficam fora do MVP (o `glow` não as mostra).

## Outcome

- dev/issues/detect-magic-bytes.md
- dev/issues/stdin.md
- dev/issues/structured-data.md
- dev/issues/html.md
- dev/issues/url.md
- dev/issues/csv.md
- dev/issues/image.md
- dev/issues/svg.md
- dev/issues/pdf.md
- dev/issues/archive.md
- dev/issues/media.md
- dev/issues/config-file.md
