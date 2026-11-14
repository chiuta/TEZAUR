# TEZAUR

[English](README.md) · [Română](README.ro.md) · **Français** · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Version : 0.5.2 · Dernière mise à jour : 2026-11-14

Un assistant de recherche local, fondé sur les citations, pour rédiger une thèse. Un seul binaire Rust.
Vos livres et articles restent sur votre machine : rien n’est envoyé nulle part, sauf si vous lancez l’une des commandes en ligne facultatives (voir « Utilisation du réseau »).
```
PDF / EPUB / DOCX / txt ──► chunk (page-tagged) ──► index (BM25 + optional embeddings)
   question or section title ──► retrieve ──► local model writes ONLY from the excerpts, with [n] citations
                                             └─► post-check flags invented citations, builds the reference list
```
## Pourquoi la recherche documentaire plutôt que « l’entraînement sur les livres »

Un ordinateur portable ne peut pas affiner (fine-tuner) un modèle sur une bibliothèque, et un modèle affiné
inventerait encore des sources. La recherche documentaire offre la garantie inverse : chaque affirmation renvoie à un passage (fichier + page) que vous
pouvez ouvrir et vérifier, et le modèle est contraint de dire ce que la bibliothèque ne contient *pas*.

## Installer depuis les dépôts de paquets (construits et publiés par la CI à chaque push)

À chaque push, ces dépôts sont reconstruits sur GitHub Pages et installés dans des conteneurs propres avant publication (voir `.github/workflows/pages.yml`).

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
Signature : les dépôts ne sont signés GPG qu’une fois que le mainteneur a ajouté le secret `GPG_PRIVATE_KEY` (voir `deploy/signing/README.md`) ; d’ici là, ils ne sont pas signés. Le mécanisme est testé en CI avec une clé jetable. Les archives de version et les fichiers `.deb` portent des attestations de provenance de build Sigstore sans clé : `gh attestation verify FILE -R chiuta/TEZAUR`. L’index F-Droid est signé avec une clé jetable générée à chaque build ; Android vous demandera donc de réinstaller l’application quand elle change.

## Tutoriels vidéo

13 courts tutoriels (application web, CLI, tableaux/équations de PDF, audit, MCP, API serveur, Mesh, installation) avec sous-titres en anglais, roumain, français, italien, espagnol, portugais, allemand et hongrois sont générés par `scripts/tutorials/make.py` (voir `scripts/tutorials/README.md`). Les vidéos se trouvent dans `assets/tutorials/` (un `.mp4` avec 8 pistes de sous-titres au choix + 8 `.vtt` chacun) et se lisent sur la page de présentation, tezaur.chiuta.ro (l’application web installable se trouve sous `/app/`). La sortie du terminal est réelle ; le modèle des démonstrations est une simulation, et les traductions sont des traductions automatiques non relues.

## Moteurs de modèles

TEZAUR a besoin d’un serveur de modèles local pour les réponses (pas pour `search`, `sources`, `ingest`).

| `--api`  | Serveur | Adresse par défaut |
|----------|---------|--------------------|
| `ollama` | Ollama | `http://localhost:11434` |
| `jan`    | [Jan](https://jan.ai) — Settings → Local API Server → Start Server | `http://127.0.0.1:1337/v1` |
| `openai` | LM Studio, llama.cpp `llama-server`, vLLM… (donnez `--host`) | `http://127.0.0.1:8080/v1` |

```bash
tezaur --api jan models            # lists the model ids Jan serves; use one as --model
tezaur --api jan --model <id> ask "…"
```
Ou placez `api = jan` et `model = <id>` dans `Data/settings/tezaur.conf` (portable) pour ne plus avoir besoin d’options.
Avec Jan/LM Studio, la fenêtre de contexte est un réglage du modèle dans ces outils, pas quelque chose que TEZAUR peut demander.
Les embeddings (`--embed-model`) ne fonctionnent que si le serveur propose un point d’accès d’embeddings ; sinon, ingest conserve
vos documents et la recherche par mots-clés, et le signale.

## Configuration (exemple Ollama)

```bash
cargo build --release                      # binary: target/release/tezaur
ollama pull llama3.1:8b                    # generation; bigger model = better and slower
ollama pull bge-m3                         # optional: multilingual embeddings (Romanian/English/French)
tezaur ingest ~/thesis/library --embed-model bge-m3
```
Entrées : **pdf, docx, epub, txt, md, html, tex**. Relancer `ingest` ne traite que les fichiers nouveaux ou modifiés ;
après une mise à jour de TEZAUR, lancez une fois `ingest --force` pour tout relire avec le nouveau découpage et les nouveaux lecteurs.
Les passages sont coupés aux fins de phrase quand c’est possible ; les EPUB sont lus dans l’ordre de lecture propre au livre.

## Commandes

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
- `--only smith` (en interactif : `/only smith`) restreint tout aux documents dont le titre, le chemin ou la
  citation correspond — les sources d’un seul chapitre à la fois, comme dans un espace de travail.
- **BibTeX** : déposez un export `.bib` de Zotero / Better BibTeX à côté de l’index (portable : n’importe où dans `Data/`).
  Un fichier est associé par clé de citation = nom de fichier, ou par le champ `file` de l’entrée. Les citations se lisent alors
  « Smith & Doe (2019), p. 3 » et la liste de références porte tous les détails. Aucune réindexation n’est nécessaire.
- **Avertissement de correspondance faible** : en mode mots-clés seuls, TEZAUR avertit quand le meilleur passage couvre moins de 40 % des
  termes de la requête, au lieu de laisser le modèle improviser à partir de rien.

Réglages (options / variables d’environnement / `tezaur.conf`) : `--api/TEZAUR_API`, `--host/TEZAUR_HOST` (Ollama aussi `OLLAMA_HOST`),
`--api-key/TEZAUR_API_KEY`, `--model/TEZAUR_MODEL`, `--embed-model/TEZAUR_EMBED_MODEL`,
`--num-ctx/TEZAUR_NUM_CTX` (Ollama), `--index/TEZAUR_INDEX`, `--only`, `--bib`.

Entrées bibliographiques et requêtes (toutes hors ligne sauf mention contraire) :

- **Formats** : outre `.bib`, les fichiers se terminant par `.ris` et `.csl.json` dans le dossier de la bibliographie sont chargés aussi
  (`--bib file.json` charge explicitement tout fichier CSL JSON). Les types de références sont mis en correspondance via CSL JSON ;
  les `@string` BibTeX, la concaténation avec `#` et l’héritage `crossref` sont lus. Les DOI, ISBN, ISSN et ORCID iD
  sont extraits et leurs chiffres de contrôle vérifiés ; `--unify-duplicates` permet aux entrées de même DOI/ISBN
  de répondre aux clés de citation les unes des autres.
- **Expressions et proximité** (facultatif) : avec `--phrases` (ou `TEZAUR_PHRASES=1`), `"exact phrase"` dans `search`, `ask` et
  l’API fait correspondre les mots dans l’ordre, et `"a b"~5` à moins de 5 mots l’un de l’autre ; les passages sans l’expression
  sont écartés, et un `~k` mal formé arrête la requête avec une erreur. Sans l’option, les guillemets sont de la ponctuation ordinaire,
  exactement comme avant, de sorte que les questions en langage naturel contenant des guillemets gardent leurs correspondances partielles.
  La correspondance ignore les accents, la casse et la forme Unicode (le cache BM25 se reconstruit une fois après la mise à jour).
- **Localisateurs** dans `audit` : `[@key, p. 3]`, `pp. 3-5`, `p. 12 ff.`, chiffres romains, étiquettes `ch.`/`sec.` ; une plage
  est vérifiée sur chaque page qu’elle nomme (`page-out-of-range`, `bad-range`).
- **EPUB** : les numéros de page imprimés (page-list / `epub:type="pagebreak"`) sont utilisés comme numéros de page quand le livre en possède.
- **Reçus** : `ask --receipt` écrit `tezaur-receipt/1`, comme avant. Définissez `TEZAUR_RECEIPT_FORMAT=2` pour écrire
  `tezaur-receipt/2` à la place (CLI, API HTTP et MCP identiquement) : l’empreinte et le HMAC facultatif (`TEZAUR_RECEIPT_KEY`)
  portent alors sur la forme canonique RFC 8785, de sorte que reformater le fichier n’a pas d’importance. Les documents sont nommés par SHA-256
  (enregistré à l’ingestion dans un nouveau fichier `<index>.sha256` à côté de l’index ; le fichier d’index lui-même est identique octet pour octet
  à avant, et le fichier annexe n’est écrit que lorsqu’un index est écrit, jamais par les commandes en lecture seule). Les documents déjà présents dans
  un index construit sans lui sont marqués « identité faible » jusqu’à une nouvelle ingestion avec `--force` ; un simple `ingest` de
  fichiers inchangés ne l’ajoute pas. `PUT`/`GET /api/index` ne copient que le fichier d’index, donc une copie synchronisée n’a pas d’identités SHA-256.
  Les deux versions de reçu se vérifient, quel que soit le format actuellement sélectionné.
- **Politesse envers Crossref** (`refs --online`, `lit`) : `--mailto you@example.org` (ou `TEZAUR_MAILTO`) place votre adresse
  dans le User-Agent pour le « polite pool » de Crossref ; les requêtes suivent les en-têtes de débit du serveur, ralentissent sur 429, s’arrêtent sur 403,
  et les réponses sont mises en cache sur disque (30 jours pour les succès, 1 jour pour les échecs) ; `--refresh-online` contourne le cache.

## Chaîne de sources (enveloppes FFF, hors ligne)

TEZAUR est la deuxième étape d’une chaîne qui mène d’un document numérisé à une référence citable sans réseau :
`legatorocr scan.png --ocr --author "Name, Given" --fff-year 1999 --fff-out doc.fff.json` -> `tezaur ingest --fff-in doc.fff.json` -> `tezaur bib --fff-out bib.fff.json` -> `veriga --fff-in bib.fff.json --fff-out checked.fff.json` -> AcademiSearch (Library -> Receive).

- `ingest --fff-in FILE` (répétable) accepte une enveloppe de type `document/ocr` (texte par page, auteur, année, SHA-256 du scan), `text/plain` ou `text/markdown`. L’enveloppe est copiée dans `fff-inbox/` à côté de l’index, une entrée BibTeX `fff-<key>.bib` est écrite à côté de l’index (auteur, année, titre, `note = sha256:…`), et les pages sont indexées ; les citations `[@key, p. N]` fonctionnent donc et `tezaur audit` peut les vérifier. Les fichiers `.fff.json` simples dans un dossier donné à `ingest` sont également indexés.
- `bib --fff-out FILE` (ou `--to fff`) écrit la bibliographie sous forme d’enveloppe `citation/csl`, pour VERIGA ou AcademiSearch.
- Limites : les erreurs d’OCR du texte sont indexées telles quelles ; l’auteur/l’année de l’entrée sont ceux fournis à l’étape précédente, non vérifiés ; le format d’enveloppe est décrit dans `docs/FFF-ENVELOPE.md` de github.com/chiuta/chiuta.github.io.

## Portable (format PortableApps.com)

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
Lancer `tezaur.exe` sans commande ouvre le mode interactif. Les documents situés dans `Data/` sont stockés par
chemin relatif, de sorte que le dossier peut se déplacer entre disques et machines. Rien n’est écrit dans le registre,
`%APPDATA%` ou le profil utilisateur. Si vous utilisez Jan plutôt qu’Ollama, Jan conserve son propre dossier de données (il peut être
déplacé dans les réglages de Jan) ; seul le côté TEZAUR est portable.

## Version HTML en un seul fichier

`tezaur.html` est la même idée dans un seul fichier hors ligne : ouvrez-le dans un navigateur, ajoutez vos fichiers, pointez-le vers Ollama / Jan /
LM Studio. Recherche BM25 (sans embeddings), ask / compare / gaps / summarize / outline / draft / flashcards, BibTeX,
vérification des citations, export Anki. L’index vit dans le navigateur (IndexedDB). Les PDF nécessitent pdf.js, récupéré une fois depuis un CDN.
Le serveur de modèles doit autoriser l’accès depuis le navigateur (CORS) : Ollama `OLLAMA_ORIGINS=*`, Jan → Local API Server → CORS.

### Autres façons de l’exécuter

Voir `variants/README.md` : serveur MCP, extension de navigateur (Chrome/Firefox), applications de bureau Electron et Tauri, `.deb`, Docker,
YunoHost, plugins pour DokuWiki / WordPress / MediaWiki / Vim / Neovim / Emacs / Sublime / Notepad++ / Obsidian / VS Code / Joplin / Logseq /
Zotero / LibreOffice / Google Docs / Calibre, bots Slack / Discord / WhatsApp / Telegram / Matrix / e-mail, clients PyPI + npm, recettes systemd / Helm /
Raspberry Pi / Termux / NixOS, et **Mesh** (échange pair à pair entre chercheurs, `variants/MESH.md`) — avec un état honnête pour chacun.

### Android / téléphone

- **`pwa/`** : la même page sous forme d’application web installable (manifeste + service worker). Placez le dossier sur n’importe quel hébergeur HTTPS
  (par exemple votre YunoHost) et utilisez Chrome → « Install app ». Fonctionne hors ligne après la première visite (page et lecture de PDF ; le serveur de modèles est à part, et la dictée du navigateur télécharge son modèle depuis Internet quand on l’utilise).
- **`android/`** : un petit projet WebView qui intègre `tezaur.html` dans un APK. Aucune chaîne d’outils locale n’est nécessaire :
  GitHub → Actions → *android-apk* → Run workflow → téléchargez `Tezaur-apk` (signé en mode débogage, installable). Ou ouvrez
  `android/` dans Android Studio. **Pas encore construit ni exécuté** (le SDK ne peut pas être téléchargé dans mon bac à sable).
  Le serveur de modèles doit être joignable depuis le téléphone (Ollama / Jan sur votre PC, même Wi-Fi) avec CORS activé.

## Des vérifications qu’aucun autre outil ne vous offre (et leurs limites)

- **`audit`** lit votre brouillon, trouve chaque `[@key, p. 3]` de style pandoc, consulte *cette page* de *cette source* et signale les phrases que la
  page ne couvre pas (`unsupported`, `weak`), une page plus probable (`wrong-page`), les clés qui ne correspondent à aucun document, les pages au-delà de la longueur du document,
  les longues affirmations sans citation, et les guillemets dont le texte n’est pas mot pour mot dans la source. C’est un test de recouvrement de termes : une paraphrase qui
  emploie d’autres mots obtiendra un score faible, et une phrase qui répète les bons mots avec le mauvais sens obtiendra un score élevé. Il vous dit où ouvrir le livre.
- **`audit --semantic`** (v0.3) demande à votre modèle local, pour chaque phrase citée, si le passage dit la même chose (SUPPORTS / PARTIAL / CONTRADICTS / UNRELATED).
  Il peut sauver une paraphrase correcte et repérer une négation que le recouvrement de mots accepte ; chaque verdict qu’il modifie est marqué `[model]` dans la note.
  Il vaut ce que vaut le modèle que vous exécutez, et il n’a été testé ici que contre un substitut scripté (voir REVIEW.md).
- **`refs`** relève de la comptabilité : clés citées qui n’existent ni dans le `.bib` ni dans la bibliothèque, entrées `.bib` et documents indexés que votre texte ne cite jamais.
  `refs --online`, `lit` et `oai` (moissonnage OAI-PMH de l’URL de dépôt que vous donnez) sont les **seules** commandes qui contactent Internet (OpenAlex, Crossref ; `TEZAUR_MAILTO` pour leur « polite pool » ;
  `TEZAUR_OPENALEX` / `TEZAUR_CROSSREF` pour pointer vers un miroir). Elles ne s’exécutent jamais implicitement et affichent un avis avant d’envoyer quoi que ce soit.
- **Tableaux et équations dans les PDF** (v0.4) : `ingest` garde un tableau en un seul passage Markdown (légende d’abord, en-tête répété s’il faut le scinder) et écrit
  les équations hors texte sous la forme `[equation] $$ LaTeX $$ (eq. N)` ; `tezaur tables FILE [--csv|--json|-o DIR]` et `tezaur equations FILE` les exportent. Les pages à deux colonnes sont lues colonne par colonne.
  C’est une **heuristique sur la couche de texte de poppler** (nécessite `pdftotext` ; sans lui, ingest se rabat sur le texte brut) : les colonnes sont retrouvées à partir d’écarts de 2 espaces ou plus, de sorte que les tableaux très serrés,
  les cellules fusionnées/multilignes, les tableaux qui sont des images, les fractions empilées, les matrices et les indices/exposants (aplatis : `pi ti`) ne sont pas reconstruits. Cela ne remplace pas un modèle d’OCR comme Nougat ou MinerU.
- **`eval`** transforme « la recherche semble correcte » en chiffres sur vos propres questions.
- L’index de recherche conserve un cache BM25 à côté du fichier d’index (`<index>.bm25`, reconstruit automatiquement quand la bibliothèque change) : une recherche CLI sur 6 M de mots est passée d’environ 4,5 s à environ 0,2 s (remesuré le 2026-10-30 sur 6 M de mots synthétiques, dans ce bac à sable ; la première recherche après un ingest paie les ~4,5 s de construction du cache).
- **`coverage`** et **`novelty`** comparent un plan ou un nouveau texte à ce que votre bibliothèque contient réellement.
- **Reçus** (`ask --receipt`) : SHA-256 sur la question, le modèle, la réponse et chaque extrait ; `receipt` le vérifie plus tard, ainsi que le fait que ces extraits existent toujours inchangés
  dans l’index. Avec `TEZAUR_RECEIPT_KEY` défini, ils portent aussi un HMAC. C’est une preuve d’altération, pas une signature d’identité, ni une preuve qu’une réponse est vraie.
  `declare` transforme les reçus en une simple déclaration d’usage de l’IA (en roumain ou en anglais) avec une ligne que vous devez confirmer vous-même.
- **Notebooks** : `%load_ext tezaur_client.magics` dans Jupyter donne `%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage` ; `python -m tezaur_client.notebook thesis.ipynb` audite la prose d’un notebook.

Voir [COMPETITION.md](COMPETITION.md) pour la comparaison avec les outils actuels, et [REVIEW.md](REVIEW.md) pour une évaluation mesurée et sans complaisance de ce qui est prouvé et de ce qui ne l’est pas (chiffres de précision de l’audit, échelle, constats de sécurité).

## Limites honnêtes

- **Pas un rédacteur de thèse.** Il rédige à partir de vos sources ; vous restez maître de l’argumentation, de la structure et de la vérification.
- La vérification des citations est mécanique : elle détecte les numéros d’extraits inexistants et les longs paragraphes non cités,
  mais ne peut pas dire si un passage cité étaye réellement une phrase. **Ouvrez la page.**
- `summarize` fait transiter les références de page à travers plusieurs appels au modèle ; TEZAUR vérifie qu’elles existent dans le document,
  pas que chacune est rattachée à la bonne phrase. Vérifiez par sondage.
- `gaps` ne voit que les extraits retrouvés : il signale ce que ces passages laissent ouvert, pas tout ce qui manque.
- Les petits modèles paraphrasent de façon approximative. Les modèles plus gros (et les exécutions plus lentes) sont nettement plus fidèles.
- Les PDF numérisés nécessitent d’abord un OCR (par exemple `ocrmypdf`) ; ils sont signalés comme « sans couche de texte ».
- EPUB/DOCX n’ont pas de numéros de page, leurs citations n’en portent donc pas. Les chapitres EPUB sont lus dans l’ordre propre au livre (spine).
- Les fragments sont des fenêtres d’environ 200 mots (40 de chevauchement) coupées en fin de phrase, sans tenir compte des chapitres. Les tableaux et équations des PDF ne sont récupérés que de façon heuristique (voir plus haut).
- La qualité de la recherche est le plafond : si `search` ne fait pas remonter le bon passage, `ask` ne peut pas l’utiliser.

- **Flashcards** ne conserve une carte que si sa réponse cite un extrait existant ; les autres sont écartées et comptées.
  Le `.tsv` comporte les lignes d’en-tête `#separator:tab` / `#html:false` et deux colonnes (recto, verso + source) ; je n’ai pas pu
  tester l’import dans Anki lui-même.
- **Les types de brouillon** changent la consigne de rédaction, pas les règles : chaque type cite toujours des extraits, et `introduction`
  laisse vos propres objectifs sous forme de marqueur `[AUTHOR: …]` au lieu de les inventer.

## D’où viennent les idées

Passés en revue : AnythingLLM, GPT4All LocalDocs, Kotaemon, Khoj, NotebookLM, PrivateGPT, les assistants fondés sur Zotero, et la
distribution Linux pour étudiants UberStudent (aujourd’hui abandonnée). D’UberStudent : un menu organisé par compétence universitaire
(lire / rechercher / écrire / étudier — la disposition de `/help`), des aides à l’étude construites autour d’Anki, des modèles pour les travaux universitaires
(`draft --kind`) et un guide de première utilisation (`doctor`, l’indication « étape suivante »). UberStudent fonctionnait aussi en direct depuis une clé USB,
ce à quoi sert la disposition portable.

## CI

- `.github/workflows/ci.yml` — à chaque push/PR : build, `cargo test`, construction de tous les plugins et exécution de toute la suite d’intégration (`variants/tests/run-all.sh`, avec poppler et pdflatex installés pour que les tests PDF s’exécutent) ; construit et teste aussi sous Windows et macOS.
- `.github/workflows/release.yml` — à la main ou sur un tag `v*` : CLI (3 systèmes), .deb, .rpm, Electron, Tauri, installeur Windows, extensions, Obsidian/VS Code/Office, APK Capacitor, et chaque plugin/intégration (`variants/build-plugins.sh`) ; sur un tag, une dernière tâche joint tous les artefacts et un `SHA256SUMS.txt` à la version GitHub.
- `portable.yml`, `android.yml`, `docker.yml`, `pages.yml` — dossier Windows PortableApps, Android, image Docker, PWA sur GitHub Pages.

**Aucun de ces workflows n’a encore tourné sur GitHub** (seules leurs étapes ont été exercées localement sous Linux) ; attendez-vous à devoir corriger un chemin ou un paquet manquant au premier lancement, surtout pour Windows, macOS, Electron, Tauri, iOS et Android.

## Délibérément non construit

- **Réenregistrer les réponses du modèle dans la bibliothèque comme sources** : la réponse suivante pourrait alors citer le propre
  texte antérieur du modèle comme s’il s’agissait d’une preuve. Enregistrez les brouillons avec `-o`, et n’ingérez-les que si vous acceptez cela.
- **Services cloud d’embeddings / de reranking** : contraires à l’objet d’un outil local. Un reranker local est un ajout
  possible à l’avenir.

## Tests

`cargo test` (61 tests, dont des vecteurs SHA-256/HMAC et la logique d’audit/de reçus) et `variants/tests/run-all.sh` — tokeniseur/diacritiques, BM25, fusion de classements, filtre de documents, découpage, lecture DOCX/EPUB, analyse
et correspondance BibTeX, analyse des citations et réécriture des pages, analyse REPL, disposition portable, et client de modèle
pour les deux protocoles (Ollama NDJSON, OpenAI SSE) face à de faux serveurs en processus.

## Licence

MIT © 2026 Alexio (https://trom.tf) — voir [LICENSE](LICENSE).

## Benchmark d’audit des citations et vérification des références

`python3 variants/tests/audit_bench.py target/release/tezaur --check` mesure l’`audit` mécanique sur 24 passages inventés x 7 types d’affirmations = 168 vérifications (mot pour mot, paraphrase, chiffre écrit autrement, chiffre ou mot modifié, négation, mauvaise source, sans rapport). Il est **synthétique et construit à partir de modèles** : il montre ce que la vérification lexicale peut et ne peut pas voir, pas la précision sur de vraies thèses. Dernière exécution :

| Type d’affirmation | Attendu | Correct | Taux |
|---|---|---|---|
| verbatim | doit être accepté | 24/24 | 100 % |
| paraphrase | envoyé à un humain (non noté) | 24/24 | 100 % |
| figure-form | doit être accepté | 22/24 | 92 % |
| figure-swap | doit être signalé | 19/19 | 100 % |
| negation | doit être signalé | 1/24 | 4 % |
| wrong-source | doit être signalé | 24/24 | 100 % |
| unrelated | doit être signalé | 24/24 | 100 % |
| word-swap | doit être signalé | 2/5 | 40 % |

Au total : rappel « signalé quand il le faut » 73 % (70/96) ; précision 97 % (70/72) ; fausses alertes sur des affirmations correctes 2/48.

Lecture : les mauvaises sources, les phrases sans rapport et les chiffres modifiés sont détectés ; les négations et les mots isolés échangés **ne le sont pas** (utilisez `audit --semantic` avec un modèle local, dont la qualité n’est pas testée avec un vrai modèle) ; les paraphrases sont envoyées à un humain par conception.

`tezaur audit draft.md --online [--fail]` vérifie en outre que chaque référence citée existe et n’est pas rétractée (OpenAlex/Crossref). C’est facultatif, cela affiche sur stderr ce qui est envoyé, et n’envoie que le DOI/titre des entrées `.bib` citées. Testé contre des services simulés ; les vrais services n’étaient pas joignables depuis le bac à sable de développement.

## Utilisation du réseau et outils externes

pdf.js 3.11.174 (Apache-2.0) est embarqué localement dans la PWA (`pwa/vendor/pdfjs`, mis en cache par son service worker), dans l’extension de navigateur (`variants/build-extension.sh` le copie et `build-web.py --local-pdfjs` y dirige la page) et dans la variante YunoHost ; le `tezaur.html` en un seul fichier et les autres enveloppes construites avec `build-web.py` le chargent depuis jsDelivr avec un hachage Subresource Integrity (le worker PDF n’est pas vérifié en intégrité). transformers.js (jsDelivr, import dynamique, SRI impossible) et le modèle Whisper tiny (Hugging Face) sont encore récupérés depuis Internet ; les bots transmettent les messages via des services tiers. « Local » s’applique donc à vos documents et à l’index de recherche, pas à chaque variante en toute circonstance. Les fonctions PDF et OCR nécessitent `pdftotext`, `pdftoppm` et `tesseract` installés. Les modèles recommandés (`llama3.1:8b`, `bge-m3`) ne sont pas fournis et portent leurs propres licences (Llama 3.1 a sa propre licence communautaire avec des restrictions d’usage). Provenance : `docs/clean-room/PROVENANCE.md`.

## Soutien

Trade-free : sans suivi, sans publicité. Si cela vous est utile, vous pouvez soutenir l’auteur : https://buymeacoffee.com/echo.of.the.strings
