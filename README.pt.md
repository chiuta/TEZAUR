# TEZAUR — Um assistente de investigação local, assente em citações, para escrever uma tese. Um único binário Rust.

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · **Português** · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versão: 0.5.2 · Última atualização: 2026-11-14

## Como funciona

Os teus livros e artigos ficam no teu computador. O TEZAUR divide-os em fragmentos etiquetados com a página, indexa-os (BM25 e, opcionalmente, embeddings), recupera as passagens para uma pergunta ou um título de secção, e um modelo local escreve apenas a partir desses excertos, com citações [n]. Uma verificação posterior assinala as citações inventadas e constrói a lista de referências.

## Comandos

- `tezaur search`: apenas passagens, sem modelo.
- `tezaur ask`: uma resposta com citações; `compare`, `gaps`, `summarize` e `outline` trabalham sobre os mesmos excertos.

## Formas de o executar

- O binário Rust na linha de comandos, com repositórios de pacotes construídos pela CI.
- Formato PortableApps.com para Windows (`build-portable.bat`).
- Um único ficheiro HTML offline, `tezaur.html`, com o índice guardado no navegador.

## Limites honestos

- Não é um redator de teses: redige a partir das tuas fontes; o argumento e a verificação são teus.
- A verificação das citações é mecânica: não consegue dizer se uma passagem citada sustenta mesmo uma frase, por isso abre a página.
- A qualidade da recuperação é o teto: se o `search` não mostrar a passagem certa, o `ask` não a pode usar.

## Privacidade

Nada sai do teu computador a menos que executes um dos comandos online opcionais (vê «Network use» no README completo).

Documentação completa (em inglês): [README.md](README.md).

## Licença

MIT — Copyright (c) 2026 Alexandru-Ionuț Chiuță (Alexio).

## Apoio

Trade-free: sem rastreio, sem anúncios. Se for útil, podes apoiar o autor: https://buymeacoffee.com/echo.of.the.strings
