# view

Roteia um arquivo, URL ou stdin para o visualizador certo e mostra o resultado
num pager (`less -RF`). Cada formato tem uma cadeia de backends; vence o
primeiro disponível no PATH.

A primeira linha do conteúdo é o comando equivalente, esmaecido, seguida de uma
régua. Se a cadeia caiu num fallback, uma linha sugere instalar a ferramenta
preferida:

```
$ bat -l md README.md
# glow não encontrado: pacman -S glow
────────────────────────────────────────
```

## Instalação

```
cargo install view-cli
```

O pacote se chama `view-cli`; o comando instalado é `view`. Os visualizadores
externos (`bat`, `glow`, `eza`, `pdftotext`...) são opcionais: sem eles, cada
cadeia cai no próximo backend.

## Uso

```
view [OPTIONS] [TARGET]
```

`TARGET` ausente ou `-` lê o stdin inteiro e detecta o formato pelo conteúdo
(magic bytes, binário ou texto; texto que parece JSON ou TOML vira JSON ou
TOML, texto que começa com `<!doctype html` ou `<html` vira HTML, e texto que
começa com `<svg` ou `<?xml` seguido de `<svg` vira SVG). Os backends recebem
os bytes por pipe e o cabeçalho mostra `-`; os que exigem caminho (`eza`, `ls`,
`bsdtar`, `tar`, `ffprobe`) recebem um arquivo temporário (modo 0600) apagado ao
final, e o cabeçalho mostra o caminho dele.

| Opção | Efeito |
| :-- | :-- |
| `-v`, `--verbose` | mostra o cabeçalho (comando equivalente, notas e régua); sem ele, só o conteúdo |
| `-n`, `--dry-run` | só imprime o comando que rodaria |
| `--raw` | json como está, sem reformatar (erro com `--as` de outro formato) |
| `--as <FORMAT>` | força o formato (`code`, `md`, `json`, `yaml`, `dir`, `binary`...); entre `json`, `yaml` e `toml` converte |
| `--no-pager` | escreve direto no stdout |

O pager vem de `$VIEW_PAGER`, depois `$PAGER`, com padrão `less -RF`. Sem
terminal no stdout, não há pager.

## Cadeias

| Tipo | Cadeia |
| :-- | :-- |
| código, texto | `bat --color=always --paging=never --style=plain` → `cat` |
| markdown | `glow -s dark -w <largura>` → `bat -l md` → `cat` |
| diretório | `eza -la --tree -L2 --color=always` → `ls -la --color=always` |
| binário | `hexyl` → `xxd` |
| json | conversão interna json→json (2 espaços, ordem das chaves mantida) → `bat -l json` → `cat`; inválido, `.jsonl`/`.ndjson` e `--raw` vão direto ao `bat` |
| yaml, toml | `bat -l <tipo>` → `cat` |
| json, yaml, toml com `--as` outro formato | conversão interna (`serde`) → `bat -l <formato>` → `cat` |
| html | conversão interna (`htmd`, sem script, estilo e comentário) → cadeia de markdown (`glow` → `bat -l md` → `cat`) |
| csv, tsv | tabela interna (`csv` + `comfy-table`) → `bat -l csv` → `cat` |
| svg | interno (`resvg`), como imagem, sem pager (ver abaixo); inválido → `bat -l xml` → `cat` |
| url | baixa e usa a cadeia do tipo da resposta (ver abaixo) |
| imagem | interno, direto no terminal, sem pager (ver abaixo) |
| pdf | `pdftotext -layout <alvo> -` |
| arquivo compactado | `bsdtar -tvf` → `tar -tvf` (só caminho) |
| áudio, vídeo | `ffprobe -hide_banner` com o stderr no stdout (só caminho) |

"Só caminho": o backend não lê stdin, então stdin e URL vão para um arquivo
temporário. Sem nenhum backend da cadeia instalado, o erro sugere como instalar
o preferido.

A detecção usa diretório, prefixo `http(s)://`, magic bytes, extensão e, por
último, o conteúdo.

## Imagens

png, jpeg, gif (primeiro quadro) e webp, de arquivo, stdin ou URL. Imagens não
passam pelo pager e `--no-pager` não muda nada: a imagem (e o cabeçalho, com
`-v`) sai direto no stdout. Com `-v`, o cabeçalho ganha `# imagem: kitty` ou `# imagem: meio-bloco`.

- kitty: com `TERM=xterm-kitty`, `KITTY_WINDOW_ID` ou `TERM_PROGRAM=ghostty` e
  stdout num terminal. Transmite o PNG com `a=T`.
- meio-bloco: o resto (inclusive pipe e redirecionamento). Usa `▀` truecolor.
- SVG: rasterizado com `resvg` (fontes do sistema) sobre fundo branco, no
  maior tamanho que cabe no terminal (pode ampliar). SVG inválido é mostrado
  como texto (`bat -l xml`), com `# svg inválido` no cabeçalho.
- Tamanho: no máximo a largura do terminal e a altura da tela menos o
  cabeçalho, na proporção da imagem (célula em pixels via `TIOCGWINSZ`; sem
  ela, 1:2). Imagem pequena não é ampliada.

## URLs

`view https://exemplo.com/pagina` baixa com `ureq` (rustls) para a memória:
limite de 20 MiB, timeout de 15 s, redirecionamentos seguidos. O tipo vem do
`Content-Type`; se ele falta ou é genérico (`text/plain`,
`application/octet-stream`, desconhecido), vale a detecção pelo conteúdo, como
no stdin. Depois o conteúdo segue o caminho do stdin (pipe ou arquivo
temporário). Com `-v`, o cabeçalho mostra o equivalente:

```
$ curl -sL https://exemplo.com/dados.json | bat --color=always --paging=never --style=plain -l json -
```

`-n` não faz requisição: sem `--as` imprime `$ curl -sL <url> | view -`, pois só
a resposta diz o tipo. Erro de rede ou de status HTTP termina com uma linha,
como `view: <url>: HTTP 404`.

## CSV e TSV

Arquivos `.csv` e `.tsv` (ou `text/csv` e `text/tab-separated-values` numa URL)
viram uma tabela na largura do terminal, com a primeira linha como cabeçalho.
O delimitador vem da extensão; sem extensão (stdin, URL), vale tab se a
primeira linha tem tab e não tem vírgula. O stdin só é tratado como CSV com
`--as csv`. Linhas com número de campos diferente são aceitas. Acima de 10 000
linhas de dados, mostra as primeiras 10 000 e uma linha avisando quantas foram
omitidas. `csvlens` e `tv` não entram na cadeia por serem interativos.

```
$ view tabela.csv  # csv→tabela interno
```

## Conversão entre json, yaml e toml

`view --as yaml foo.json` converte internamente (a ordem das chaves é mantida)
e passa o resultado para `bat -l yaml`. Vale para arquivo e stdin; sem
extensão reconhecida, o formato de origem é adivinhado pelo conteúdo. O
cabeçalho mostra os dois passos:

```
$ view --as yaml foo.json  # json→yaml interno | bat --color=always --paging=never --style=plain -l yaml
```

Conversão impossível (ex. `null` para toml) termina com erro, sem saída.
`--as` igual ao formato de origem não converte.

## Languages

Messages and `--help` are available in English, Brazilian Portuguese (pt-BR)
and Spanish (es).

- The language comes from `LC_ALL`, `LC_MESSAGES` and `LANG`, in this order:
  the first one that is set and not empty decides. `pt*` selects pt-BR, `es*`
  selects Spanish, `en*` and anything else (`C`, `POSIX`, `de_DE`...) selects
  English. There is no `--lang`; to force a language use `LC_ALL=es_ES.UTF-8 view ...`.
- To add a language, create `locales/<tag>/view.ftl` with the same messages as
  `locales/en/view.ftl` (Fluent syntax), and add one line to `LANGUAGES` in
  `src/i18n.rs`. A test checks that all files have the same message ids.
- Known limitation: the labels clap generates ("Usage:", "Options:", "Print
  help") and its argument parsing errors stay in English. The `# ...` notes
  are translated, but the `$ ...` command of the `-v` header and the output of
  external tools (`bat`, `pdftotext`...) are not.

## Licença

MIT ou Apache-2.0, à escolha.

## Desenvolvimento

`just check` roda formatação, clippy e testes. Veja `AGENTS.md`.
