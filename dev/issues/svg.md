---
type: issue
status: todo
tags: [svg, image]
relatedTo: [dev/rfc/mvp.md, dev/issues/image.md]
kind: enhancement
created: 2026-10-15
creator: claude
---

# SVG rasterizado no terminal

- R1. Rasterizar com o crate `resvg` (fontes do sistema) e mostrar pelo mesmo
  caminho das imagens: kitty ou meio-bloco, direto no terminal, sem pager.
- R2. Por ser vetorial, o SVG é rasterizado já no tamanho que cabe no
  terminal (pode ampliar), mantendo a proporção.
- R3. Fundo branco: a maioria dos SVGs supõe página branca, e texto escuro
  some num terminal escuro.
- R4. SVG inválido cai no `bat -l xml` sem erro, como o json inválido.
- R5. Stdin que começa com `<svg` (ou `<?xml` seguido de `<svg`) é SVG.
- R6. Cabeçalho com `-v`: `# imagem: kitty` ou `# imagem: meio-bloco`.

## Critérios de aceitação

- Teste com `tests/examples/drawing.svg` em pipe (meio-bloco).
- Teste de SVG inválido mostrado como texto.
- Teste do stdin detectado como SVG.

## Plano

- [ ] Dependência `resvg` e rasterização no tamanho do terminal, fundo branco (R1, R2, R3)
- [ ] `Internal::Svg` escreve direto no terminal, com o cabeçalho de imagem (R1, R6)
- [ ] SVG inválido cai no `bat -l xml` (R4)
- [ ] Detecção de SVG no stdin (R5)
- [ ] Testes, `README.md` e issue em `in_review`
