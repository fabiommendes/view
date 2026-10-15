---
type: issue
status: in_review
tags: [json, yaml, toml]
relatedTo: [dev/rfc/mvp.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# Conversão entre json, yaml e toml com --as

`view --as yaml foo.json` mostra o json como yaml, e assim entre os três.

- R1. Parse para `serde_json::Value` (`preserve_order`), serializa no formato
  alvo, passa a string para `bat -l <formato>` (fallback `cat`).
- R2. YAML com `serde-saphyr`; TOML com `toml` (`preserve_order`). Antes de
  adotar o `serde-saphyr`, confirme num teste que ele preserva a ordem das
  chaves ao serializar um `Value`.
- R3. Conversão impossível (ex. `null` para TOML) dá erro claro, sem pânico.
- R4. Sem `--as`, ou com `--as` igual ao formato de origem, nada muda: `bat`.
- R5. Cabeçalho: `$ view --as yaml foo.json  # json→yaml interno | bat -l yaml`.

## Critérios de aceitação

- Testes de ida e volta json→yaml→json preservam valores e ordem das chaves.
- Teste de erro para `null` em TOML.

## Plano

- [x] Dependências e teste de ordem de chaves do `serde-saphyr` (R2).
- [x] Módulo `convert` (parse e serialização json, yaml, toml) com erros claros (R1, R2, R3).
- [x] Backend `Pipe`: renderizador interno cujos bytes alimentam outro backend; `Internal::Convert` (R1, R5).
- [x] Rota: `--as` entre formatos estruturados converte; igual à origem não muda (R4).
- [x] Testes de integração (pares de conversão, stdin, erro de `null` em TOML) e `README.md`.
