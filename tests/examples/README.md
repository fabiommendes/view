# Exemplos de arquivo por tipo

Arquivos de exemplo para validar o `view` à mão e como entrada de testes.
Rode `view <arquivo>` e confira o tipo e o que está na tabela.

O tipo é decidido em `src/detect.rs`, nesta ordem: URL e diretório, magic
bytes, extensão e, por fim, texto ou binário pelo conteúdo.

| arquivo | tipo esperado | o que observar |
| :-- | :-- | :-- |
| `code.rs` | código | destaque de sintaxe Rust; comentários e strings coloridos |
| `code.py` | código | docstrings e decoradores; `format_brl` com `divmod` |
| `code.ts` | código | interface, `??=`, função `async` e um `console.log` no fim |
| `notes.txt` | texto | texto corrido em UTF-8 com acentos; cabeçalho e régua |
| `LICENSE` | texto | sem extensão; cai em texto |
| `readme.md` | markdown | títulos, lista, tabela, bloco de código, link e citação |
| `page.html` | html | `<title>`, `<h1>`, lista, link e tabela; o `<script>` e o `<style>` não aparecem |
| `data.json` | json | objeto aninhado; `null` em `descricao` e `pager` |
| `data-no-null.json` | json | igual ao anterior sem `null`; base para converter em TOML |
| `config.yaml` | yaml | comentário no fim da linha e lista em linha `[5, 10, 30]` |
| `config.toml` | toml | tabelas `[servidor]`, `[banco]` e `[logs]` |
| `table.csv` | csv | campo `"Belo Horizonte, MG"` com vírgula dentro das aspas não quebra a coluna |
| `table.tsv` | csv | tabulações separam colunas; cai na mesma rota do CSV |
| `image.png` | imagem | gradiente 64x48; o backend interno ainda não existe, então a saída atual é a da cadeia de fallback |
| `photo.jpg` | imagem | mesmo gradiente em JPEG |
| `image-as.txt` | imagem (pelo conteúdo) | PNG com extensão `.txt`; os magic bytes vencem a extensão |
| `drawing.svg` | svg | retângulo, círculo, triângulo e texto |
| `doc.pdf` | pdf | uma página com texto simples |
| `archive.zip` | arquivo compactado | lista `code.py`, `notes.txt` e `readme.md` |
| `archive.tar.gz` | arquivo compactado | lista `page.html`, `data.json` e `config.yaml` |
| `binary.bin` | binário | 512 bytes aleatórios com nulos; hexdump (`hexyl` ou `xxd`) |
| `program` | binário (pelo conteúdo) | ELF sem extensão; sem extensão, o conteúdo decide |
| `sound.wav` | mídia | 1 s de tom a 8 kHz, mono |
| `dir/` | diretório | `a.txt`, `b.md` e o subdiretório `sub/` com `c.txt` |
