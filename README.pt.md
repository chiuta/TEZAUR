# TEZAUR

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · **Português** · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versão: 0.5.2 · Última atualização: 2026-11-14

Um assistente de investigação local, baseado em citações, para escrever uma tese. Um único binário Rust.
Os seus livros e artigos ficam no seu computador: nada é enviado para lado nenhum, a não ser que execute um dos comandos online opcionais (veja «Utilização da rede»).
```
PDF / EPUB / DOCX / txt ──► chunk (page-tagged) ──► index (BM25 + optional embeddings)
   question or section title ──► retrieve ──► local model writes ONLY from the excerpts, with [n] citations
                                             └─► post-check flags invented citations, builds the reference list
```
## Porquê recuperação e não «treinar com os livros»

Um portátil não consegue fazer fine-tuning de um modelo com uma biblioteca, e um modelo afinado
continuaria a inventar fontes. A recuperação dá a garantia oposta: cada afirmação aponta para uma passagem (ficheiro + página) que pode
abrir e verificar, e o modelo é obrigado a dizer o que a biblioteca *não* contém.

## Instalar a partir dos repositórios de pacotes (compilados e publicados pela CI a cada push)

A cada push, estes repositórios são reconstruídos no GitHub Pages e instalados a partir deles em contentores limpos antes de publicar (veja `.github/workflows/pages.yml`).

```bash
BASE=https://chiuta.github.io/TEZAUR
# Debian / Ubuntu  -- if $BASE/apt/tezaur.gpg exists the repository is signed (preferred):
curl -fsSL $BASE/apt/tezaur.gpg | sudo tee /usr/share/keyrings/tezaur.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/tezaur.gpg] $BASE/apt ./" | sudo tee /etc/apt/sources.list.d/tezaur.list && sudo apt update && sudo apt install tezaur
#   (unsigned fallback: echo "deb [trusted=yes] $BASE/apt ./" ...)
# Fedora / RHEL    (the .repo file sets gpgcheck=1 when the repository is signed, gpgcheck=0 otherwise -- read it before installing)
sudo curl -o /etc/yum.repos.d/tezaur.repo $BASE/rpm/tezaur.repo && sudo dnf install tezaur
# Flatpak
flatpak remote-add --user tezaur $BASE/flatpak/tezaur.flatpakrepo && flatpak install tezaur tf.trom.Tezaur
# Android: add  $BASE/fdroid/repo  as a repository in the F-Droid app
```
Assinatura: os repositórios só são assinados com GPG depois de o mantenedor acrescentar o segredo `GPG_PRIVATE_KEY` (veja `deploy/signing/README.md`); até lá não estão assinados. O mecanismo é testado na CI com uma chave descartável. Os arquivos de versão e os ficheiros `.deb` trazem atestações de proveniência de compilação Sigstore, sem chaves: `gh attestation verify FILE -R chiuta/TEZAUR`. O índice do F-Droid é assinado com uma chave descartável gerada em cada compilação, por isso o Android pedir-lhe-á que reinstale quando ela mudar.

## Tutoriais em vídeo

13 tutoriais curtos (aplicação web, CLI, tabelas/equações de PDF, auditoria, MCP, API do servidor, Mesh, instalação) com legendas em inglês, romeno, francês, italiano, espanhol, português, alemão e húngaro são gerados por `scripts/tutorials/make.py` (veja `scripts/tutorials/README.md`). Os vídeos estão em `assets/tutorials/` (um `.mp4` com 8 pistas de legendas selecionáveis + 8 `.vtt` cada) e reproduzem-se na página de apresentação, tezaur.chiuta.ro (a aplicação web instalável está em `/app/`). A saída do terminal é real; o modelo das demonstrações é simulado, e as traduções são traduções automáticas não revistas.

## Motores de modelos

O TEZAUR precisa de um servidor de modelos local para as respostas (não para `search`, `sources`, `ingest`).

| `--api`  | Servidor | Endereço predefinido |
|----------|----------|----------------------|
| `ollama` | Ollama | `http://localhost:11434` |
| `jan`    | [Jan](https://jan.ai) — Settings → Local API Server → Start Server | `http://127.0.0.1:1337/v1` |
| `openai` | LM Studio, llama.cpp `llama-server`, vLLM… (indique `--host`) | `http://127.0.0.1:8080/v1` |

```bash
tezaur --api jan models            # lists the model ids Jan serves; use one as --model
tezaur --api jan --model <id> ask "…"
```
Ou ponha `api = jan` e `model = <id>` em `Data/settings/tezaur.conf` (portátil) para dispensar as opções.
Com o Jan/LM Studio, a janela de contexto é uma definição do modelo nesses programas, não algo que o TEZAUR possa pedir.
Os embeddings (`--embed-model`) só funcionam se o servidor oferecer um endpoint de embeddings; se não, o ingest mantém
os seus documentos e a pesquisa por palavras-chave, e avisa disso.

## Configuração (exemplo com Ollama)

```bash
cargo build --release                      # binary: target/release/tezaur
ollama pull llama3.1:8b                    # generation; bigger model = better and slower
ollama pull bge-m3                         # optional: multilingual embeddings (Romanian/English/French)
tezaur ingest ~/thesis/library --embed-model bge-m3
```
Entradas: **pdf, docx, epub, txt, md, html, tex**. Voltar a executar `ingest` só processa os ficheiros novos ou alterados;
depois de atualizar o TEZAUR, execute `ingest --force` uma vez para reler tudo com a nova divisão em fragmentos e os novos leitores.
As passagens são cortadas nos limites das frases sempre que possível; os EPUB são lidos na ordem de leitura própria do livro.

## Comandos

```bash
tezaur search "burden of proof"                  # passages only, no model; shows % of query terms matched
tezaur ask "Who bears the burden of proof?"      # cited answer;  --decompose first splits the question into queries
tezaur compare "How is X defined?"               # positions, agreements, disagreements across sources
tezaur gaps "recognition of qualifications"      # what is established / weak / unanswered / missing
tezaur summarize "smith2019"                     # whole-document summary (map-reduce, slow, page references)
tezaur outline "Recognition of qualifications"   # chapter skeleton, marks sections lacking sources
tezaur draft "Principles" --notes $'burden\ncompensation' -o draft.md --lang română
tezaur draft "Recognition" --kind lit-review     # also: introduction, critical-review (default: section)
tezaur flashcards "burden of proof" -o cards.tsv # study cards; each answer must cite a source; imports into Anki
tezaur doctor [--generate]                       # checks server, model, embeddings, index, BibTeX; says what to fix
tezaur sources                                   # what is indexed (+ BibTeX citation)
tezaur serve                                     # local web server: web app + API + OCR + model proxy (variants/README.md)
tezaur export draft.md --to tex|docx|html|pdf|odt|epub   # LaTeX with \cite{} (+ --bib-name), Word, HTML; pdf needs pdflatex, odt/epub need pandoc
tezaur bib --to ris|csl-json|bibtex              # your bibliography for Zotero, Mendeley, pandoc, Word
tezaur bib --duplicates [--json]                 # entries that share a DOI or ISBN (reported, never merged unless --unify-duplicates)
tezaur oai https://repo.example/oai --prefix oai_dc [--set S] [--from D] [--until D] [--max N] [--max-requests N] [--incremental] [-o out.bib [--force]]
                                                 # opt-in ONLINE harvest of an OAI-PMH repository into BibTeX (records from the repository leave nothing of yours); an existing -o file is refused unless --incremental (append) or --force (replace); stops after --max-requests (default 200) or when the archive repeats a resumption token / keeps sending empty pages
tezaur mesh share smith; tezaur mesh add ana http://192.168.1.20:8787 --token X; tezaur mesh search "burden of proof"; tezaur mesh pull
                                                 # peer-to-peer exchange with colleagues, opt-in (variants/MESH.md)
tezaur audit draft.md [--fail]                   # check each [@key, p. 3] against the cited page; quotes word-for-word; no model needed
tezaur audit draft.md --semantic                 # + second opinion from your local model per citation (paraphrases, negations); slow, can be wrong
tezaur refs draft.md [--online]                  # cited keys found nowhere, .bib/library entries never cited; --online checks DOIs/retractions (OpenAlex, Crossref)
tezaur lit "mutual recognition" [--bibtex]       # opt-in ONLINE literature search (OpenAlex, Crossref fallback); the query leaves your computer
tezaur eval cases.jsonl                          # recall@1/3/10 + MRR of the search on YOUR library: {"q": "...", "doc": "key", "page": 12}
tezaur coverage outline.md                       # which sections have sources (well-covered / thin / empty)
tezaur novelty paper.pdf                         # what a text adds beyond your library (covered / partly / new)
tezaur ask "…" --receipt r.json; tezaur receipt r.json; tezaur declare r.json --lang ro
                                                 # tamper-evident receipt per answer, and an AI-use statement from receipts
tezaur zotero                                    # BibTeX from Zotero's local API + its attachments
tezaur ask "…" --rerank --verify                 # model re-orders passages; model checks each cited sentence
tezaur mcp                                       # MCP server on stdio: your library as tools in Claude Desktop/Code (see variants/README.md)
tezaur                                           # no command: interactive mode (/ingest, /only, free-text questions)
```
- `--only smith` (em modo interativo: `/only smith`) restringe tudo aos documentos cujo título, caminho ou
  citação corresponda — as fontes de um só capítulo de cada vez, como num espaço de trabalho.
- **BibTeX**: ponha uma exportação `.bib` do Zotero / Better BibTeX junto ao índice (portátil: em qualquer lugar de `Data/`).
  Um ficheiro é associado pela chave de citação = nome do ficheiro, ou pelo campo `file` da entrada. As citações passam então a ler-se
  «Smith & Doe (2019), p. 3» e a lista de referências traz os detalhes completos. Não é preciso reindexar.
- **Aviso de correspondência fraca**: no modo só de palavras-chave, o TEZAUR avisa quando a melhor passagem cobre menos de 40 % dos
  termos da consulta, em vez de deixar o modelo improvisar a partir do nada.

Definições (opções / variáveis de ambiente / `tezaur.conf`): `--api/TEZAUR_API`, `--host/TEZAUR_HOST` (Ollama também `OLLAMA_HOST`),
`--api-key/TEZAUR_API_KEY`, `--model/TEZAUR_MODEL`, `--embed-model/TEZAUR_EMBED_MODEL`,
`--num-ctx/TEZAUR_NUM_CTX` (Ollama), `--index/TEZAUR_INDEX`, `--only`, `--bib`.

Entradas bibliográficas e consultas (tudo offline, salvo indicação em contrário):

- **Formatos**: além de `.bib`, também são carregados os ficheiros terminados em `.ris` e `.csl.json` da pasta da bibliografia
  (`--bib file.json` carrega explicitamente qualquer ficheiro CSL JSON). Os tipos de referência são mapeados através de CSL JSON;
  são lidos os `@string` do BibTeX, a concatenação com `#` e a herança `crossref`. DOI, ISBN, ISSN e ORCID iD
  são extraídos e os seus dígitos de controlo verificados; `--unify-duplicates` permite que entradas com o mesmo DOI/ISBN
  respondam às chaves de citação umas das outras.
- **Frases e proximidade** (opcional): com `--phrases` (ou `TEZAUR_PHRASES=1`), `"exact phrase"` em `search`, `ask` e
  na API procura as palavras por ordem, e `"a b"~5` a não mais de 5 palavras de distância; as passagens sem a frase
  são descartadas, e um `~k` malformado interrompe a consulta com um erro. Sem a opção, as aspas são pontuação normal,
  exatamente como antes, pelo que as perguntas em linguagem natural que contêm aspas mantêm as suas correspondências parciais.
  A correspondência ignora acentos, maiúsculas/minúsculas e forma Unicode (a cache BM25 é reconstruída uma vez após a atualização).
- **Localizadores** em `audit`: `[@key, p. 3]`, `pp. 3-5`, `p. 12 ff.`, algarismos romanos, etiquetas `ch.`/`sec.`; um intervalo
  é verificado em cada página que nomeia (`page-out-of-range`, `bad-range`).
- **EPUB**: os números de página impressos (page-list / `epub:type="pagebreak"`) são usados como números de página quando o livro os tem.
- **Recibos**: `ask --receipt` escreve `tezaur-receipt/1`, como antes. Defina `TEZAUR_RECEIPT_FORMAT=2` para escrever
  em vez disso `tezaur-receipt/2` (CLI, API HTTP e MCP por igual): o resumo e o HMAC opcional (`TEZAUR_RECEIPT_KEY`)
  são então calculados sobre a forma canónica RFC 8785, pelo que reformatar o ficheiro não importa. Os documentos são identificados por SHA-256
  (registado na ingestão num novo ficheiro `<index>.sha256` junto ao índice; o ficheiro de índice em si é idêntico byte a byte
  ao de antes, e o ficheiro acessório só é escrito quando se escreve um índice, nunca pelos comandos só de leitura). Os documentos já existentes em
  um índice construído sem ele ficam marcados como «identidade fraca» até serem reingeridos com `--force`; um simples `ingest` de
  ficheiros inalterados não o acrescenta. `PUT`/`GET /api/index` copiam apenas o ficheiro de índice, pelo que uma cópia sincronizada não tem identidades SHA-256.
  Ambas as versões de recibo se verificam, seja qual for o formato atualmente selecionado.
- **Cortesia com a Crossref** (`refs --online`, `lit`): `--mailto you@example.org` (ou `TEZAUR_MAILTO`) coloca o seu endereço
  no User-Agent para o «polite pool» da Crossref; os pedidos seguem os cabeçalhos de ritmo do servidor, recuam perante 429, param perante 403,
  e as respostas ficam em cache no disco (30 dias para os acertos, 1 dia para as falhas); `--refresh-online` ignora a cache.

## Cadeia de fontes (envelopes FFF, offline)

O TEZAUR é o segundo passo de uma cadeia que leva um documento digitalizado a uma referência citável sem rede:
`legatorocr scan.png --ocr --author "Name, Given" --fff-year 1999 --fff-out doc.fff.json` -> `tezaur ingest --fff-in doc.fff.json` -> `tezaur bib --fff-out bib.fff.json` -> `veriga --fff-in bib.fff.json --fff-out checked.fff.json` -> AcademiSearch (Library -> Receive).

- `ingest --fff-in FILE` (repetível) aceita um envelope do tipo `document/ocr` (texto por página, autor, ano, SHA-256 da digitalização), `text/plain` ou `text/markdown`. O envelope é copiado para `fff-inbox/` junto ao índice, é escrita uma entrada BibTeX `fff-<key>.bib` junto ao índice (autor, ano, título, `note = sha256:…`) e as páginas são indexadas, pelo que as citações `[@key, p. N]` funcionam e o `tezaur audit` pode verificá-las. Os ficheiros `.fff.json` simples numa pasta dada ao `ingest` também são indexados.
- `bib --fff-out FILE` (ou `--to fff`) escreve a bibliografia como um envelope `citation/csl`, para o VERIGA ou o AcademiSearch.
- Limites: os erros de OCR no texto são indexados tal como estão; o autor/ano da entrada são os que foram dados ao passo anterior, não verificados; o formato do envelope está em `docs/FFF-ENVELOPE.md` em github.com/chiuta/chiuta.github.io.

## Portátil (formato PortableApps.com)

```
build-portable.bat      # Windows: builds and assembles dist\TezaurPortable   (build-portable.sh on Linux/macOS)
```

```
TezaurPortable/
  TezaurPortable.exe            launcher (src/launcher.rs): sets TEZAUR_DATA, starts bundled Ollama if present
  help.html
  App/AppInfo/appinfo.ini, appicon*.png/.ico, Launcher/TezaurPortable.ini   (PortableApps.com metadata)
  App/Tezaur/tezaur.exe         the application
  App/Ollama/ollama.exe         OPTIONAL: unzip the official portable Ollama here
  Data/                         everything that is yours: tezaur.idx, Library/, settings/tezaur.conf, Output/, ollama/models
  Other/Source/
```
Executar `tezaur.exe` sem comando abre o modo interativo. Os documentos dentro de `Data/` são guardados por
caminho relativo, pelo que a pasta pode mudar-se entre unidades e máquinas. Nada é escrito no registo,
em `%APPDATA%` nem no perfil do utilizador. Se usar o Jan em vez do Ollama, o Jan mantém a sua própria pasta de dados (pode ser
mudada nas definições do Jan); só o lado do TEZAUR é portátil.

## Versão HTML de um só ficheiro

O `tezaur.html` é a mesma ideia num único ficheiro offline: abra-o num navegador, adicione os seus ficheiros, aponte-o para o Ollama / Jan /
LM Studio. Pesquisa BM25 (sem embeddings), ask / compare / gaps / summarize / outline / draft / flashcards, BibTeX,
verificação de citações, exportação para o Anki. O índice vive no navegador (IndexedDB). Os PDF precisam do pdf.js, obtido uma vez a partir de uma CDN.
O servidor de modelos tem de permitir o acesso a partir do navegador (CORS): Ollama `OLLAMA_ORIGINS=*`, Jan → Local API Server → CORS.

### Outras formas de o executar

Veja `variants/README.md`: servidor MCP, extensão de navegador (Chrome/Firefox), aplicações de ambiente de trabalho Electron e Tauri, `.deb`, Docker,
YunoHost, plugins para DokuWiki / WordPress / MediaWiki / Vim / Neovim / Emacs / Sublime / Notepad++ / Obsidian / VS Code / Joplin / Logseq /
Zotero / LibreOffice / Google Docs / Calibre, bots de Slack / Discord / WhatsApp / Telegram / Matrix / e-mail, clientes PyPI + npm, receitas de systemd / Helm /
Raspberry Pi / Termux / NixOS, e **Mesh** (troca entre pares entre investigadores, `variants/MESH.md`) — com um estado honesto para cada um.

### Android / telemóvel

- **`pwa/`**: a mesma página como aplicação web instalável (manifesto + service worker). Ponha a pasta em qualquer alojamento HTTPS
  (por exemplo o seu YunoHost) e use o Chrome → «Install app». Funciona offline após a primeira visita (página e leitura de PDF; o servidor de modelos é separado, e o ditado do navegador obtém o seu modelo da internet quando é usado).
- **`android/`**: um pequeno projeto WebView que embute o `tezaur.html` num APK. Não é preciso nenhuma cadeia de ferramentas local:
  GitHub → Actions → *android-apk* → Run workflow → descarregue `Tezaur-apk` (assinado em modo de depuração, instalável). Ou abra
  `android/` no Android Studio. **Ainda não foi compilado nem executado** (o SDK não pode ser descarregado no meu ambiente isolado).
  O servidor de modelos tem de ser acessível a partir do telemóvel (Ollama / Jan no seu PC, a mesma rede Wi-Fi) com CORS ativado.

## Verificações que nenhuma outra ferramenta lhe dá (e os seus limites)

- **`audit`** lê o seu rascunho, encontra cada `[@key, p. 3]` ao estilo pandoc, consulta *essa página* de *essa fonte* e comunica as frases que a
  página não cobre (`unsupported`, `weak`), uma página mais provável (`wrong-page`), chaves que não correspondem a nenhum documento, páginas para além do comprimento do documento,
  afirmações longas sem citação e aspas cujo texto não está palavra por palavra na fonte. É um teste de sobreposição de termos: uma paráfrase que
  use outras palavras terá pontuação baixa, e uma frase que repita as palavras certas com o sentido errado terá pontuação alta. Diz-lhe onde abrir o livro.
- **`audit --semantic`** (v0.3) pergunta ao seu modelo local, para cada frase citada, se a passagem diz o mesmo (SUPPORTS / PARTIAL / CONTRADICTS / UNRELATED).
  Pode salvar uma paráfrase correta e apanhar uma negação que a sobreposição de palavras aceita; cada veredicto que altera é marcado `[model]` na nota.
  Só é tão bom quanto o modelo que executar, e aqui só foi testado com um substituto guionizado (veja REVIEW.md).
- **`refs`** é contabilidade: chaves citadas que não existem nem no `.bib` nem na biblioteca, entradas `.bib` e documentos indexados que o seu texto nunca cita.
  `refs --online`, `lit` e `oai` (recolha OAI-PMH a partir do URL do repositório que indicar) são os **únicos** comandos que contactam a internet (OpenAlex, Crossref; `TEZAUR_MAILTO` para o «polite pool» deles;
  `TEZAUR_OPENALEX` / `TEZAUR_CROSSREF` para apontar para um espelho). Nunca são executados implicitamente e imprimem um aviso antes de enviar o que quer que seja.
- **Tabelas e equações em PDF** (v0.4): o `ingest` mantém uma tabela junta como uma única passagem Markdown (primeiro a legenda, cabeçalho repetido se tiver de ser dividida) e escreve
  as equações destacadas como `[equation] $$ LaTeX $$ (eq. N)`; `tezaur tables FILE [--csv|--json|-o DIR]` e `tezaur equations FILE` exportam-nas. As páginas de duas colunas são lidas coluna a coluna.
  É uma **heurística sobre a camada de texto do poppler** (precisa de `pdftotext`; sem ele, o ingest recorre a texto simples): as colunas são recuperadas a partir de intervalos de 2 ou mais espaços, pelo que as tabelas muito apertadas,
  as células fundidas/de várias linhas, as tabelas que são imagens, as frações empilhadas, as matrizes e os índices/expoentes (achatados: `pi ti`) não são reconstruídos. Não substitui um modelo de OCR como o Nougat ou o MinerU.
- **`eval`** transforma «a pesquisa parece estar bem» em números sobre as suas próprias perguntas.
- O índice de pesquisa mantém uma cache BM25 junto ao ficheiro de índice (`<index>.bm25`, reconstruída automaticamente quando a biblioteca muda): uma pesquisa CLI sobre 6 M de palavras passou de ~4,5 s para ~0,2 s (voltou a medir-se em 2026-10-30 sobre 6 M de palavras sintéticas, neste ambiente; a primeira pesquisa após um ingest paga os ~4,5 s de construção da cache).
- **`coverage`** e **`novelty`** comparam um esquema ou um texto novo com o que a sua biblioteca realmente contém.
- **Recibos** (`ask --receipt`): SHA-256 sobre a pergunta, o modelo, a resposta e cada excerto; `receipt` verifica-o mais tarde, bem como o facto de esses excertos ainda existirem inalterados
  no índice. Com `TEZAUR_RECEIPT_KEY` definida, trazem também um HMAC. Isto é evidência de adulteração, não uma assinatura de identidade, nem prova de que uma resposta é verdadeira.
  `declare` transforma os recibos numa simples declaração de utilização de IA (em romeno ou inglês) com uma linha que tem de confirmar você mesmo.
- **Notebooks**: `%load_ext tezaur_client.magics` no Jupyter dá `%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage`; `python -m tezaur_client.notebook thesis.ipynb` audita a prosa de um notebook.

Veja [COMPETITION.md](COMPETITION.md) para saber como se compara com as ferramentas atuais, e [REVIEW.md](REVIEW.md) para uma avaliação medida e sem lisonja do que está e do que não está demonstrado (números de precisão da auditoria, escala, constatações de segurança).

## Limites honestos

- **Não é um escritor de teses.** Redige a partir das suas fontes; o argumento, a estrutura e a verificação são seus.
- A verificação de citações é mecânica: apanha números de excerto inexistentes e parágrafos longos sem citação,
  mas não consegue dizer se uma passagem citada sustenta realmente uma frase. **Abra a página.**
- `summarize` transporta as referências de página através de várias chamadas ao modelo; o TEZAUR verifica que existem no documento,
  não que cada uma esteja ligada à frase certa. Verifique por amostragem.
- `gaps` só vê os excertos recuperados: comunica o que essas passagens deixam em aberto, não tudo o que falta.
- Os modelos pequenos parafraseiam com frouxidão. Os modelos maiores (e as execuções mais lentas) são nitidamente mais fiéis.
- Os PDF digitalizados precisam primeiro de OCR (por exemplo `ocrmypdf`); são comunicados como «sem camada de texto».
- EPUB/DOCX não têm números de página, por isso as suas citações não levam página. Os capítulos EPUB são lidos na ordem própria do livro (spine).
- Os fragmentos são janelas de ~200 palavras (40 de sobreposição) cortadas no fim das frases, sem atender aos capítulos. As tabelas e equações dos PDF só são recuperadas de forma heurística (veja acima).
- A qualidade da recuperação é o teto: se `search` não fizer surgir a passagem certa, `ask` não a pode usar.

- **Flashcards** só mantém um cartão se a sua resposta citar um excerto que exista; os restantes são descartados e contados.
  O `.tsv` tem as linhas de cabeçalho `#separator:tab` / `#html:false` e duas colunas (frente, verso + fonte); não consegui
  testar a importação no próprio Anki.
- **Os tipos de rascunho** alteram o pedido de redação, não as regras: todos os tipos continuam a citar excertos, e `introduction`
  deixa os seus próprios objetivos como um marcador `[AUTHOR: …]` em vez de os inventar.

## De onde vieram as ideias

Analisados: AnythingLLM, GPT4All LocalDocs, Kotaemon, Khoj, NotebookLM, PrivateGPT, os assistentes baseados no Zotero e a
(agora abandonada) distribuição Linux para estudantes UberStudent. Do UberStudent: um menu organizado por competência académica
(ler / investigar / escrever / estudar — a disposição de `/help`), auxiliares de estudo construídos em torno do Anki, modelos para trabalhos académicos
(`draft --kind`) e um guia de primeira utilização (`doctor`, a sugestão do «passo seguinte»). O UberStudent também funcionava em modo live a partir de uma pen USB,
que é para o que serve a disposição portátil.

## CI

- `.github/workflows/ci.yml` — a cada push/PR: compilação, `cargo test`, compila todos os plugins e executa toda a suite de integração (`variants/tests/run-all.sh`, com poppler e pdflatex instalados para que os testes de PDF corram); também compila e testa em Windows e macOS.
- `.github/workflows/release.yml` — à mão ou numa etiqueta `v*`: CLI (3 sistemas operativos), .deb, .rpm, Electron, Tauri, instalador Windows, extensões, Obsidian/VS Code/Office, APK Capacitor e todos os plugins/integrações (`variants/build-plugins.sh`); numa etiqueta, uma tarefa final anexa todos os artefactos e um `SHA256SUMS.txt` à versão do GitHub.
- `portable.yml`, `android.yml`, `docker.yml`, `pages.yml` — pasta Windows PortableApps, Android, imagem Docker, PWA no GitHub Pages.

**Nenhum destes fluxos de trabalho correu ainda no GitHub** (apenas os seus passos foram exercitados localmente em Linux); espere ter de corrigir um caminho ou um pacote em falta na primeira execução, sobretudo em Windows, macOS, Electron, Tauri, iOS e Android.

## Deliberadamente não construído

- **Guardar as respostas do modelo de volta na biblioteca como fontes**: a resposta seguinte poderia então citar o próprio
  texto anterior do modelo como se fosse prova. Guarde os rascunhos com `-o` e só os ingira se aceitar isso.
- **Serviços na nuvem de embeddings / reranking**: contrários ao propósito de uma ferramenta local. Um reranker local é uma possível
  adição futura.

## Testes

`cargo test` (61 testes, incluindo vetores SHA-256/HMAC e a lógica de auditoria/recibos) e `variants/tests/run-all.sh` — tokenizador/diacríticos, BM25, fusão de classificações, filtro de documentos, divisão em fragmentos, leitura de DOCX/EPUB, análise
e correspondência de BibTeX, análise de citações e reescrita de páginas, análise do REPL, disposição portátil e cliente de modelos
para ambos os protocolos (Ollama NDJSON, OpenAI SSE) contra servidores falsos em processo.

## Licença

MIT © 2026 Alexio (https://trom.tf) — veja [LICENSE](LICENSE).

## Referência de auditoria de citações e verificação de referências

`python3 variants/tests/audit_bench.py target/release/tezaur --check` mede a `audit` mecânica sobre 24 passagens inventadas x 7 tipos de afirmação = 168 verificações (literal, paráfrase, algarismo escrito de outra forma, algarismo ou palavra alterados, negação, fonte errada, sem relação). É **sintético e construído a partir de modelos**: mostra o que a verificação lexical consegue e não consegue ver, não a precisão em teses reais. Última execução:

| Tipo de afirmação | Esperado | Correto | Taxa |
|---|---|---|---|
| verbatim | deve ser aceite | 24/24 | 100 % |
| paraphrase | enviado a uma pessoa (não pontuado) | 24/24 | 100 % |
| figure-form | deve ser aceite | 22/24 | 92 % |
| figure-swap | deve ser assinalado | 19/19 | 100 % |
| negation | deve ser assinalado | 1/24 | 4 % |
| wrong-source | deve ser assinalado | 24/24 | 100 % |
| unrelated | deve ser assinalado | 24/24 | 100 % |
| word-swap | deve ser assinalado | 2/5 | 40 % |

No conjunto: abrangência de «assinalado quando deve ser» 73 % (70/96); precisão 97 % (70/72); falsos alarmes em afirmações corretas 2/48.

Como ler: as fontes erradas, as frases sem relação e os algarismos alterados são apanhados; as negações e as palavras isoladas trocadas **não** são (use `audit --semantic` com um modelo local, cuja qualidade não foi testada com um modelo real); as paráfrases são enviadas a uma pessoa por conceção.

`tezaur audit draft.md --online [--fail]` verifica ainda que cada referência citada existe e não foi retratada (OpenAlex/Crossref). É opcional, imprime em stderr o que envia e envia apenas o DOI/título das entradas `.bib` citadas. Testado com serviços simulados; os serviços reais não eram acessíveis a partir do ambiente isolado de desenvolvimento.

## Utilização da rede e ferramentas externas

O pdf.js 3.11.174 (Apache-2.0) está incluído localmente na PWA (`pwa/vendor/pdfjs`, guardado em cache pelo seu service worker), na extensão de navegador (`variants/build-extension.sh` copia-o e `build-web.py --local-pdfjs` aponta a página para ele) e na variante YunoHost; o `tezaur.html` de um só ficheiro e os outros invólucros construídos com `build-web.py` carregam-no a partir do jsDelivr com um hash de Subresource Integrity (o worker de PDF não tem verificação de integridade). O transformers.js (jsDelivr, importação dinâmica, sem SRI possível) e o modelo Whisper tiny (Hugging Face) ainda são obtidos da internet; os bots reencaminham as mensagens através de serviços de terceiros. «Local» aplica-se, portanto, aos seus documentos e ao índice de recuperação, não a todas as variantes em todos os momentos. As funcionalidades de PDF e OCR precisam de `pdftotext`, `pdftoppm` e `tesseract` instalados. Os modelos recomendados (`llama3.1:8b`, `bge-m3`) não estão incluídos e têm as suas próprias licenças (o Llama 3.1 tem uma licença comunitária própria com restrições de utilização). Proveniência: `docs/clean-room/PROVENANCE.md`.

## Apoio

Trade-free: sem rastreio, sem anúncios. Se lhe for útil, pode apoiar o autor: https://buymeacoffee.com/echo.of.the.strings
