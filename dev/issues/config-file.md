---
type: issue
status: deferred
tags: [config]
relatedTo: [dev/rfc/mvp.md, src/route.rs]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Arquivo de configuração com as cadeias

Último item da segunda leva do RFC `mvp`. Precisa de decisões do humano antes
do plano.

## Perguntas

- Escopo: só trocar a ordem ou remover backends da cadeia embutida, ou também
  declarar backends externos novos (programa, argumentos, dica)?
- Formato e lugar: `~/.config/view/config.toml` (`$XDG_CONFIG_HOME`)?
- Casos de uso concretos que motivam o arquivo.

## Decisão

Adiada em 2026-10-15 pelo humano: sem caso de uso concreto. Reabrir se surgir
a necessidade.
