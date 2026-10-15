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

## Uso

```
view [OPTIONS] [TARGET]
```

`TARGET` ausente ou `-` lê o stdin inteiro e detecta o formato pelo conteúdo
(magic bytes, binário ou texto; texto que parece JSON ou TOML vira JSON ou
TOML, e texto que começa com `<!doctype html` ou `<html` vira HTML). Os backends recebem os bytes por pipe e o cabeçalho mostra `-`; `eza` e
`ls`, que exigem caminho, recebem um arquivo temporário (modo 0600) apagado ao
final, e o cabeçalho mostra o caminho dele.

| Opção | Efeito |
| :-- | :-- |
| `-q`, `--quiet` | sem cabeçalho |
| `-n`, `--dry-run` | só imprime o comando que rodaria |
| `--as <FORMAT>` | força o formato (`code`, `md`, `json`, `yaml`, `dir`, `binary`...); entre `json`, `yaml` e `toml` converte |
| `--no-pager` | escreve direto no stdout |

O pager vem de `$VIEW_PAGER`, depois `$PAGER`, com padrão `less -RF`. Sem
terminal no stdout, não há pager.

## Cadeias

Backends internos (Rust) ainda não implementados são pulados.

| Tipo | Cadeia |
| :-- | :-- |
| código, texto | `bat --color=always --paging=never --style=plain` → `cat` |
| markdown | `glow -s dark -w <largura>` → `bat -l md` → `cat` |
| diretório | `eza -la --tree -L2 --color=always` → `ls -la --color=always` |
| binário | `hexyl` → `xxd` |
| json, yaml, toml | `bat -l <tipo>` → `cat` |
| json, yaml, toml com `--as` outro formato | conversão interna (`serde`) → `bat -l <formato>` → `cat` |
| html | conversão interna (`htmd`, sem script, estilo e comentário) → cadeia de markdown (`glow` → `bat -l md` → `cat`) |
| csv, tsv | tabela interna (`csv` + `comfy-table`) → `bat -l csv` → `cat` |
| svg | interno (pendente) → `bat -l svg` → `cat` |
| url | baixa e usa a cadeia do tipo da resposta (ver abaixo) |
| imagem, pdf, arquivo compactado, mídia | interno (pendente) |

A detecção usa diretório, prefixo `http(s)://`, magic bytes, extensão e, por
último, o conteúdo.

## URLs

`view https://exemplo.com/pagina` baixa com `ureq` (rustls) para a memória:
limite de 20 MiB, timeout de 15 s, redirecionamentos seguidos. O tipo vem do
`Content-Type`; se ele falta ou é genérico (`text/plain`,
`application/octet-stream`, desconhecido), vale a detecção pelo conteúdo, como
no stdin. Depois o conteúdo segue o caminho do stdin (pipe ou arquivo
temporário). O cabeçalho mostra o equivalente:

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

## Desenvolvimento

`just check` roda formatação, clippy e testes. Veja `AGENTS.md`.
