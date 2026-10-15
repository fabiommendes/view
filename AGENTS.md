# AGENTS.md

Orientações para agentes de código de IA (e para pessoas) que trabalham neste repositório.

## Idioma

Nível 1 (repositório privado), conforme `~/git/tools/tooling-root/dev/adr/0002-idioma-dos-repositorios.md`:

- Em português (pt-BR): documentos, comentários no código e mensagens de commit.
- Em inglês: identificadores.

## O que é

`view` é uma CLI em Rust que roteia um arquivo, URL ou stdin para o visualizador certo e mostra o resultado num pager (`less -RF`). Cada formato tem uma cadeia ordenada de backends; vence o primeiro disponível. O primeiro conteúdo paginado é um cabeçalho com o comando equivalente.

## Comandos

- checagem (define "pronto"): `just check` (`cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test`)
- execução local: `cargo run -- [OPTIONS] [TARGET]`
- imagem no terminal: `cargo run -- tests/examples/photo.jpg` (o `view` não usa pager para imagens)

## Mapa

- `src/detect.rs`: `Input`, `Kind` e detecção (URL, diretório, magic bytes, extensão, conteúdo).
- `src/stdin.rs`: leitura de stdin e arquivo temporário (guard com `Drop`) para backends que exigem caminho.
- `src/backend.rs`: `Backend` (externo ou interno) e resolução da cadeia.
- `src/route.rs`: tabela `Kind` → cadeia de backends.
- `src/header.rs`: linha do comando, dica de instalação e régua.
- `src/pager.rs`: trait `Pager`, pager externo e escrita direta.
- `src/image.rs`: imagens direto no terminal (kitty `a=T` ou meio-bloco), tamanho via `TIOCGWINSZ`.
- `dev/`: `issues/`, `adr/` e `notes/`, conforme o ADR 0003 do tooling-root.

## Regras

- Backends externos escrevem com cor num pipe que alimenta o nosso pager; nunca ative o pager deles.
- Imagem não passa pelo pager: `Internal::writes_to_terminal` marca renderizadores que escrevem direto no stdout, depois do cabeçalho. Veja `dev/notes/kitty-placeholders.md`.
- Renderizadores internos (`backend::Internal`) ficam fora da resolução enquanto `is_implemented` for `false`. Ao implementar um, troque o `todo!()` e o `is_implemented`.
- Toda cadeia nova em `route.rs` vem com teste e entra na tabela do `README.md`.
- Sem crates novos sem necessidade clara.
- Trabalhe direto na `main`, sem branch nem worktree (decisão do humano, desvio do ADR 0003). Uma issue ainda passa por `in_progress` e `in_review`, e cada item do `## Plano` é marcado no commit que o fecha.

## Ordens permanentes

Manutenções que um agente faz sem issue aprovada, desde que `just check` passe:

- Corrigir avisos novos do clippy.
- Atualizar dependências em versões compatíveis (`cargo update`).
- Consertar um teste instável sem mudar o comportamento testado.
- Corrigir links e caminhos quebrados na documentação.
