# TEZAUR

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · **Italiano** · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versione: 0.5.2 · Ultimo aggiornamento: 2026-11-14

Un assistente di ricerca locale, ancorato alle citazioni, per scrivere una tesi. Un solo binario Rust.
I tuoi libri e articoli restano sul tuo computer: nulla viene inviato altrove a meno che tu non esegua uno dei comandi online facoltativi (vedi «Uso della rete»).
```
PDF / EPUB / DOCX / txt ──► chunk (page-tagged) ──► index (BM25 + optional embeddings)
   question or section title ──► retrieve ──► local model writes ONLY from the excerpts, with [n] citations
                                             └─► post-check flags invented citations, builds the reference list
```
## Perché il recupero e non «l’addestramento sui libri»

Un portatile non può fare il fine-tuning di un modello su una biblioteca, e un modello affinato
inventerebbe comunque fonti. Il recupero offre la garanzia opposta: ogni affermazione rimanda a un passaggio (file + pagina) che puoi
aprire e verificare, e il modello è costretto a dire ciò che la biblioteca *non* contiene.

## Installare dai repository di pacchetti (costruiti e pubblicati dalla CI a ogni push)

A ogni push questi repository vengono ricostruiti su GitHub Pages e installati in container puliti prima della pubblicazione (vedi `.github/workflows/pages.yml`).

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
Firma: i repository sono firmati con GPG solo dopo che il manutentore aggiunge il segreto `GPG_PRIVATE_KEY` (vedi `deploy/signing/README.md`); fino ad allora non sono firmati. Il meccanismo è testato in CI con una chiave usa e getta. Gli archivi di release e i file `.deb` portano attestazioni di provenienza della build Sigstore senza chiavi: `gh attestation verify FILE -R chiuta/TEZAUR`. L’indice F-Droid è firmato con una chiave usa e getta generata a ogni build, quindi Android ti chiederà di reinstallare l’app quando cambia.

## Video tutorial

13 brevi tutorial (app web, CLI, tabelle/equazioni dei PDF, audit, MCP, API del server, Mesh, installazione) con sottotitoli in inglese, rumeno, francese, italiano, spagnolo, portoghese, tedesco e ungherese sono generati da `scripts/tutorials/make.py` (vedi `scripts/tutorials/README.md`). I video si trovano in `assets/tutorials/` (un `.mp4` con 8 tracce di sottotitoli selezionabili + 8 `.vtt` ciascuno) e si guardano nella pagina di presentazione, tezaur.chiuta.ro (l’app web installabile si trova sotto `/app/`). L’output del terminale è reale; il modello nelle demo è simulato e le traduzioni sono traduzioni automatiche non riviste.

## Backend dei modelli

TEZAUR ha bisogno di un server di modelli locale per le risposte (non per `search`, `sources`, `ingest`).

| `--api`  | Server | Indirizzo predefinito |
|----------|--------|-----------------------|
| `ollama` | Ollama | `http://localhost:11434` |
| `jan`    | [Jan](https://jan.ai) — Settings → Local API Server → Start Server | `http://127.0.0.1:1337/v1` |
| `openai` | LM Studio, llama.cpp `llama-server`, vLLM… (indica `--host`) | `http://127.0.0.1:8080/v1` |

```bash
tezaur --api jan models            # lists the model ids Jan serves; use one as --model
tezaur --api jan --model <id> ask "…"
```
Oppure inserisci `api = jan` e `model = <id>` in `Data/settings/tezaur.conf` (portabile) così le opzioni non servono più.
Con Jan/LM Studio la finestra di contesto è un’impostazione del modello lì, non qualcosa che TEZAUR possa richiedere.
Gli embedding (`--embed-model`) funzionano solo se il server offre un endpoint per gli embedding; in caso contrario ingest conserva
i tuoi documenti e la ricerca per parole chiave, e lo segnala.

## Configurazione (esempio con Ollama)

```bash
cargo build --release                      # binary: target/release/tezaur
ollama pull llama3.1:8b                    # generation; bigger model = better and slower
ollama pull bge-m3                         # optional: multilingual embeddings (Romanian/English/French)
tezaur ingest ~/thesis/library --embed-model bge-m3
```
Input: **pdf, docx, epub, txt, md, html, tex**. Rieseguire `ingest` elabora solo i file nuovi o modificati;
dopo aver aggiornato TEZAUR, esegui una volta `ingest --force` per rileggere tutto con la nuova suddivisione in frammenti e i nuovi lettori.
I passaggi vengono tagliati ai confini delle frasi quando possibile; gli EPUB vengono letti nell’ordine di lettura proprio del libro.

## Comandi

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
- `--only smith` (in modalità interattiva: `/only smith`) limita tutto ai documenti il cui titolo, percorso o
  citazione corrisponde — le fonti di un solo capitolo alla volta, come in uno spazio di lavoro.
- **BibTeX**: metti un’esportazione `.bib` di Zotero / Better BibTeX accanto all’indice (portabile: ovunque in `Data/`).
  Un file viene abbinato per chiave di citazione = nome del file, oppure tramite il campo `file` della voce. Le citazioni si leggono allora
  «Smith & Doe (2019), p. 3» e l’elenco dei riferimenti riporta i dettagli completi. Non serve reindicizzare.
- **Avviso di corrispondenza debole**: nella modalità a sole parole chiave TEZAUR avvisa quando il miglior passaggio copre meno del 40% dei
  termini della query, invece di lasciare che il modello improvvisi sul nulla.

Impostazioni (opzioni / variabili d’ambiente / `tezaur.conf`): `--api/TEZAUR_API`, `--host/TEZAUR_HOST` (per Ollama anche `OLLAMA_HOST`),
`--api-key/TEZAUR_API_KEY`, `--model/TEZAUR_MODEL`, `--embed-model/TEZAUR_EMBED_MODEL`,
`--num-ctx/TEZAUR_NUM_CTX` (Ollama), `--index/TEZAUR_INDEX`, `--only`, `--bib`.

Input bibliografici e query (tutto offline salvo indicazione contraria):

- **Formati**: oltre a `.bib`, vengono caricati anche i file che terminano in `.ris` e `.csl.json` nella cartella della bibliografia
  (`--bib file.json` carica esplicitamente qualsiasi file CSL JSON). I tipi di riferimento sono mappati tramite CSL JSON;
  vengono letti `@string` di BibTeX, la concatenazione con `#` e l’ereditarietà `crossref`. DOI, ISBN, ISSN e ORCID iD
  vengono estratti e le loro cifre di controllo verificate; `--unify-duplicates` permette alle voci con lo stesso DOI/ISBN
  di rispondere alle chiavi di citazione delle altre.
- **Frasi e prossimità** (facoltativo): con `--phrases` (o `TEZAUR_PHRASES=1`), `"exact phrase"` in `search`, `ask` e
  nell’API cerca le parole in ordine, e `"a b"~5` entro 5 parole l’una dall’altra; i passaggi senza la frase
  vengono scartati e un `~k` malformato interrompe la query con un errore. Senza l’opzione le virgolette sono normale punteggiatura,
  esattamente come prima, quindi le domande in linguaggio naturale che contengono virgolette mantengono le corrispondenze parziali.
  La corrispondenza ignora accenti, maiuscole/minuscole e forma Unicode (la cache BM25 si ricostruisce una volta dopo l’aggiornamento).
- **Localizzatori** in `audit`: `[@key, p. 3]`, `pp. 3-5`, `p. 12 ff.`, numeri romani, etichette `ch.`/`sec.`; un intervallo
  viene controllato su ogni pagina che nomina (`page-out-of-range`, `bad-range`).
- **EPUB**: i numeri di pagina stampati (page-list / `epub:type="pagebreak"`) vengono usati come numeri di pagina quando il libro li ha.
- **Ricevute**: `ask --receipt` scrive `tezaur-receipt/1`, come prima. Imposta `TEZAUR_RECEIPT_FORMAT=2` per scrivere
  invece `tezaur-receipt/2` (CLI, API HTTP e MCP allo stesso modo): l’impronta e l’HMAC facoltativo (`TEZAUR_RECEIPT_KEY`)
  sono allora calcolati sulla forma canonica RFC 8785, quindi riformattare il file non importa. I documenti sono identificati da SHA-256
  (registrato all’ingestione in un nuovo file `<index>.sha256` accanto all’indice; il file indice in sé è identico byte per byte
  a prima, e il file accessorio viene scritto solo quando si scrive un indice, mai dai comandi di sola lettura). I documenti già presenti in
  un indice costruito senza di esso sono segnati come «identità debole» finché non vengono reingeriti con `--force`; un semplice `ingest` di
  file invariati non lo aggiunge. `PUT`/`GET /api/index` copiano solo il file indice, quindi una copia sincronizzata non ha identità SHA-256.
  Entrambe le versioni di ricevuta si verificano, qualunque sia il formato attualmente selezionato.
- **Cortesia verso Crossref** (`refs --online`, `lit`): `--mailto you@example.org` (o `TEZAUR_MAILTO`) mette il tuo indirizzo
  nello User-Agent per il «polite pool» di Crossref; le richieste seguono le intestazioni di frequenza del server, rallentano su 429, si fermano su 403,
  e le risposte vengono messe in cache su disco (30 giorni per i riscontri, 1 giorno per i mancati); `--refresh-online` aggira la cache.

## Catena delle fonti (buste FFF, offline)

TEZAUR è il secondo passo di una catena che porta un documento scansionato a un riferimento citabile senza rete:
`legatorocr scan.png --ocr --author "Name, Given" --fff-year 1999 --fff-out doc.fff.json` -> `tezaur ingest --fff-in doc.fff.json` -> `tezaur bib --fff-out bib.fff.json` -> `veriga --fff-in bib.fff.json --fff-out checked.fff.json` -> AcademiSearch (Library -> Receive).

- `ingest --fff-in FILE` (ripetibile) accetta una busta di tipo `document/ocr` (testo per pagina, autore, anno, SHA-256 della scansione), `text/plain` o `text/markdown`. La busta viene copiata in `fff-inbox/` accanto all’indice, una voce BibTeX `fff-<key>.bib` viene scritta accanto all’indice (autore, anno, titolo, `note = sha256:…`) e le pagine vengono indicizzate, quindi le citazioni `[@key, p. N]` funzionano e `tezaur audit` può controllarle. Anche i semplici file `.fff.json` in una cartella passata a `ingest` vengono indicizzati.
- `bib --fff-out FILE` (o `--to fff`) scrive la bibliografia come busta `citation/csl`, per VERIGA o AcademiSearch.
- Limiti: gli errori di OCR nel testo vengono indicizzati così come sono; autore/anno della voce sono quelli forniti al passo precedente, non verificati; il formato della busta è descritto in `docs/FFF-ENVELOPE.md` in github.com/chiuta/chiuta.github.io.

## Portabile (formato PortableApps.com)

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
Eseguire `tezaur.exe` senza comando apre la modalità interattiva. I documenti dentro `Data/` sono memorizzati con
percorso relativo, quindi la cartella può spostarsi tra unità e computer. Non viene scritto nulla nel registro,
in `%APPDATA%` o nel profilo utente. Se usi Jan invece di Ollama, Jan mantiene la propria cartella di dati (può essere
spostata nelle impostazioni di Jan); solo il lato TEZAUR è portabile.

## Versione HTML in un solo file

`tezaur.html` è la stessa idea in un unico file offline: aprilo in un browser, aggiungi i tuoi file, puntalo a Ollama / Jan /
LM Studio. Ricerca BM25 (senza embedding), ask / compare / gaps / summarize / outline / draft / flashcards, BibTeX,
controllo delle citazioni, esportazione Anki. L’indice vive nel browser (IndexedDB). I PDF richiedono pdf.js, scaricato una volta da una CDN.
Il server di modelli deve consentire l’accesso dal browser (CORS): Ollama `OLLAMA_ORIGINS=*`, Jan → Local API Server → CORS.

### Altri modi di eseguirlo

Vedi `variants/README.md`: server MCP, estensione per browser (Chrome/Firefox), app desktop Electron e Tauri, `.deb`, Docker,
YunoHost, plugin per DokuWiki / WordPress / MediaWiki / Vim / Neovim / Emacs / Sublime / Notepad++ / Obsidian / VS Code / Joplin / Logseq /
Zotero / LibreOffice / Google Docs / Calibre, bot per Slack / Discord / WhatsApp / Telegram / Matrix / e-mail, client PyPI + npm, ricette systemd / Helm /
Raspberry Pi / Termux / NixOS, e **Mesh** (scambio peer-to-peer tra ricercatori, `variants/MESH.md`) — con uno stato onesto per ciascuno.

### Android / telefono

- **`pwa/`**: la stessa pagina come app web installabile (manifest + service worker). Metti la cartella su qualsiasi host HTTPS
  (per esempio il tuo YunoHost) e usa Chrome → «Install app». Funziona offline dopo la prima visita (pagina e lettura dei PDF; il server di modelli è separato e la dettatura del browser scarica il proprio modello da Internet quando viene usata).
- **`android/`**: un piccolo progetto WebView che incorpora `tezaur.html` in un APK. Non serve alcuna toolchain locale:
  GitHub → Actions → *android-apk* → Run workflow → scarica `Tezaur-apk` (firmato in modalità debug, installabile). Oppure apri
  `android/` in Android Studio. **Non ancora compilato né eseguito** (l’SDK non può essere scaricato nel mio sandbox).
  Il server di modelli deve essere raggiungibile dal telefono (Ollama / Jan sul tuo PC, stessa rete Wi-Fi) con CORS abilitato.

## Controlli che nessun altro strumento ti offre (e i loro limiti)

- **`audit`** legge la tua bozza, trova ogni `[@key, p. 3]` in stile pandoc, cerca *quella pagina* di *quella fonte* e segnala le frasi che la
  pagina non copre (`unsupported`, `weak`), una pagina più probabile (`wrong-page`), le chiavi che non corrispondono ad alcun documento, le pagine oltre la lunghezza del documento,
  le affermazioni lunghe senza citazione e le virgolette il cui testo non è parola per parola nella fonte. È un test di sovrapposizione di termini: una parafrasi che
  usa altre parole otterrà un punteggio basso, e una frase che ripete le parole giuste con il significato sbagliato otterrà un punteggio alto. Ti dice dove aprire il libro.
- **`audit --semantic`** (v0.3) chiede al tuo modello locale, per ogni frase citata, se il passaggio dice la stessa cosa (SUPPORTS / PARTIAL / CONTRADICTS / UNRELATED).
  Può salvare una parafrasi corretta e cogliere una negazione che la sovrapposizione di parole accetta; ogni verdetto che modifica è segnato `[model]` nella nota.
  Vale quanto il modello che esegui, ed è stato provato qui solo con un sostituto scriptato (vedi REVIEW.md).
- **`refs`** è contabilità: chiavi citate che non esistono né nel `.bib` né nella biblioteca, voci `.bib` e documenti indicizzati che il tuo testo non cita mai.
  `refs --online`, `lit` e `oai` (raccolta OAI-PMH dall’URL del repository che indichi) sono gli **unici** comandi che contattano Internet (OpenAlex, Crossref; `TEZAUR_MAILTO` per il loro «polite pool»;
  `TEZAUR_OPENALEX` / `TEZAUR_CROSSREF` per puntare a uno specchio). Non vengono mai eseguiti implicitamente e stampano un avviso prima di inviare qualsiasi cosa.
- **Tabelle ed equazioni nei PDF** (v0.4): `ingest` mantiene una tabella unita come un unico passaggio Markdown (prima la didascalia, intestazione ripetuta se va divisa) e scrive
  le equazioni in evidenza come `[equation] $$ LaTeX $$ (eq. N)`; `tezaur tables FILE [--csv|--json|-o DIR]` e `tezaur equations FILE` le esportano. Le pagine a due colonne vengono lette colonna per colonna.
  È un’**euristica sullo strato di testo di poppler** (richiede `pdftotext`; senza, ingest ripiega sul testo semplice): le colonne vengono recuperate da spazi vuoti di 2 o più spazi, quindi le tabelle molto strette,
  le celle unite/su più righe, le tabelle che sono immagini, le frazioni impilate, le matrici e pedici/apici (appiattiti: `pi ti`) non vengono ricostruiti. Non sostituisce un modello OCR come Nougat o MinerU.
- **`eval`** trasforma «la ricerca sembra a posto» in numeri sulle tue domande.
- L’indice di ricerca mantiene una cache BM25 accanto al file indice (`<index>.bm25`, ricostruita automaticamente quando la biblioteca cambia): una ricerca CLI su 6 M di parole è passata da ~4,5 s a ~0,2 s (rimisurato il 2026-10-30 su 6 M di parole sintetiche, in questo sandbox; la prima ricerca dopo un ingest paga i ~4,5 s di costruzione della cache).
- **`coverage`** e **`novelty`** confrontano una scaletta o un nuovo testo con ciò che la tua biblioteca contiene davvero.
- **Ricevute** (`ask --receipt`): SHA-256 su domanda, modello, risposta e ogni estratto; `receipt` lo verifica in seguito, insieme al fatto che quegli estratti esistano ancora invariati
  nell’indice. Con `TEZAUR_RECEIPT_KEY` impostata portano anche un HMAC. È una prova di manomissione, non una firma d’identità, e non prova che una risposta sia vera.
  `declare` trasforma le ricevute in una semplice dichiarazione di uso dell’IA (in rumeno o inglese) con una riga che devi confermare tu stesso.
- **Notebook**: `%load_ext tezaur_client.magics` in Jupyter offre `%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage`; `python -m tezaur_client.notebook thesis.ipynb` controlla la prosa di un notebook.

Vedi [COMPETITION.md](COMPETITION.md) per il confronto con gli strumenti attuali e [REVIEW.md](REVIEW.md) per una valutazione misurata e priva di indulgenza di ciò che è e non è dimostrato (numeri di accuratezza dell’audit, scala, rilievi di sicurezza).

## Limiti onesti

- **Non è uno scrittore di tesi.** Redige a partire dalle tue fonti; l’argomentazione, la struttura e la verifica restano tue.
- Il controllo delle citazioni è meccanico: coglie numeri di estratto inesistenti e lunghi paragrafi non citati,
  ma non può dire se un passaggio citato sostenga davvero una frase. **Apri la pagina.**
- `summarize` porta i riferimenti di pagina attraverso più chiamate al modello; TEZAUR controlla che esistano nel documento,
  non che ciascuno sia legato alla frase giusta. Verifica a campione.
- `gaps` vede solo gli estratti recuperati: segnala ciò che quei passaggi lasciano aperto, non tutto ciò che manca.
- I modelli piccoli parafrasano in modo approssimativo. I modelli più grandi (e le esecuzioni più lente) sono sensibilmente più fedeli.
- I PDF scansionati richiedono prima l’OCR (ad esempio `ocrmypdf`); vengono segnalati come «nessuno strato di testo».
- EPUB/DOCX non hanno numeri di pagina, quindi le loro citazioni non riportano la pagina. I capitoli EPUB vengono letti nell’ordine proprio del libro (spine).
- I frammenti sono finestre di ~200 parole (40 di sovrapposizione) tagliate a fine frase, senza tener conto dei capitoli. Le tabelle e le equazioni nei PDF vengono recuperate solo in modo euristico (vedi sopra).
- La qualità del recupero è il tetto: se `search` non fa emergere il passaggio giusto, `ask` non può usarlo.

- **Flashcards** conserva una scheda solo se la sua risposta cita un estratto esistente; le altre vengono scartate e contate.
  Il `.tsv` ha le righe di intestazione `#separator:tab` / `#html:false` e due colonne (fronte, retro + fonte); non ho potuto
  provare l’importazione in Anki stesso.
- **I tipi di bozza** cambiano le istruzioni di scrittura, non le regole: ogni tipo cita comunque estratti, e `introduction`
  lascia i tuoi obiettivi come segnaposto `[AUTHOR: …]` invece di inventarli.

## Da dove vengono le idee

Esaminati: AnythingLLM, GPT4All LocalDocs, Kotaemon, Khoj, NotebookLM, PrivateGPT, gli assistenti basati su Zotero e la
distribuzione Linux per studenti UberStudent (ora abbandonata). Da UberStudent: un menu organizzato per competenza accademica
(leggere / ricercare / scrivere / studiare — la disposizione di `/help`), ausili allo studio costruiti attorno ad Anki, modelli per i lavori accademici
(`draft --kind`) e una guida al primo avvio (`doctor`, il suggerimento «prossimo passo»). UberStudent funzionava anche in modalità live da una chiavetta USB,
ed è a questo che serve la disposizione portabile.

## CI

- `.github/workflows/ci.yml` — a ogni push/PR: build, `cargo test`, costruisce tutti i plugin ed esegue l’intera suite di integrazione (`variants/tests/run-all.sh`, con poppler e pdflatex installati così che i test PDF vengano eseguiti); costruisce e testa anche su Windows e macOS.
- `.github/workflows/release.yml` — a mano o su un tag `v*`: CLI (3 sistemi operativi), .deb, .rpm, Electron, Tauri, installer Windows, estensioni, Obsidian/VS Code/Office, APK Capacitor e ogni plugin/integrazione (`variants/build-plugins.sh`); su un tag, un job finale allega tutti gli artefatti e un `SHA256SUMS.txt` alla release di GitHub.
- `portable.yml`, `android.yml`, `docker.yml`, `pages.yml` — cartella Windows PortableApps, Android, immagine Docker, PWA su GitHub Pages.

**Nessuno di questi workflow è ancora stato eseguito su GitHub** (solo i loro passi sono stati provati in locale su Linux); aspettati di dover correggere un percorso o un pacchetto mancante alla prima esecuzione, soprattutto per Windows, macOS, Electron, Tauri, iOS e Android.

## Volutamente non realizzato

- **Salvare le risposte del modello nella biblioteca come fonti**: la risposta successiva potrebbe allora citare il testo
  precedente del modello come se fosse una prova. Salva le bozze con `-o` e ingeriscile solo se accetti questo.
- **Servizi cloud di embedding / reranking**: contrari allo scopo di uno strumento locale. Un reranker locale è una possibile
  aggiunta futura.

## Test

`cargo test` (61 test, inclusi vettori SHA-256/HMAC e la logica di audit/ricevute) e `variants/tests/run-all.sh` — tokenizzatore/segni diacritici, BM25, fusione dei ranking, filtro dei documenti, suddivisione in frammenti, lettura DOCX/EPUB, parsing
e abbinamento BibTeX, parsing delle citazioni e riscrittura delle pagine, parsing REPL, disposizione portabile e client del modello
per entrambi i protocolli (Ollama NDJSON, OpenAI SSE) contro finti server in-process.

## Licenza

MIT © 2026 Alexio (https://trom.tf) — vedi [LICENSE](LICENSE).

## Benchmark dell’audit delle citazioni e controllo dei riferimenti

`python3 variants/tests/audit_bench.py target/release/tezaur --check` misura l’`audit` meccanico su 24 passaggi inventati x 7 tipi di affermazione = 168 controlli (parola per parola, parafrasi, cifra scritta in altro modo, cifra o parola cambiata, negazione, fonte sbagliata, non correlata). È **sintetico e costruito da modelli**: mostra ciò che il controllo lessicale può e non può vedere, non l’accuratezza su tesi reali. Ultima esecuzione:

| Tipo di affermazione | Atteso | Corretto | Tasso |
|---|---|---|---|
| verbatim | da accettare | 24/24 | 100 % |
| paraphrase | inviato a una persona (non valutato) | 24/24 | 100 % |
| figure-form | da accettare | 22/24 | 92 % |
| figure-swap | da segnalare | 19/19 | 100 % |
| negation | da segnalare | 1/24 | 4 % |
| wrong-source | da segnalare | 24/24 | 100 % |
| unrelated | da segnalare | 24/24 | 100 % |
| word-swap | da segnalare | 2/5 | 40 % |

Nel complesso: richiamo «segnalato quando dovrebbe esserlo» 73 % (70/96); precisione 97 % (70/72); falsi allarmi su affermazioni corrette 2/48.

Come leggerlo: fonti sbagliate, frasi non correlate e cifre cambiate vengono colte; le negazioni e le singole parole scambiate **no** (usa `audit --semantic` con un modello locale, la cui qualità non è testata con un modello reale); le parafrasi vengono inviate a una persona per progetto.

`tezaur audit draft.md --online [--fail]` verifica inoltre che ogni riferimento citato esista e non sia ritrattato (OpenAlex/Crossref). È facoltativo, stampa su stderr ciò che invia e invia solo il DOI/titolo delle voci `.bib` citate. Testato contro servizi simulati; i servizi reali non erano raggiungibili dal sandbox di sviluppo.

## Uso della rete e strumenti esterni

pdf.js 3.11.174 (Apache-2.0) è incluso localmente nella PWA (`pwa/vendor/pdfjs`, messo in cache dal suo service worker), nell’estensione per browser (`variants/build-extension.sh` lo copia e `build-web.py --local-pdfjs` vi indirizza la pagina) e nella variante YunoHost; il `tezaur.html` in un solo file e gli altri involucri costruiti con `build-web.py` lo caricano da jsDelivr con un hash Subresource Integrity (il worker PDF non è controllato per integrità). transformers.js (jsDelivr, import dinamico, nessun SRI possibile) e il modello Whisper tiny (Hugging Face) vengono ancora scaricati da Internet; i bot inoltrano i messaggi tramite servizi di terzi. «Locale» vale quindi per i tuoi documenti e per l’indice di recupero, non per ogni variante in ogni momento. Le funzioni PDF e OCR richiedono `pdftotext`, `pdftoppm` e `tesseract` installati. I modelli consigliati (`llama3.1:8b`, `bge-m3`) non sono inclusi e hanno le proprie licenze (Llama 3.1 ha una propria licenza della community con restrizioni d’uso). Provenienza: `docs/clean-room/PROVENANCE.md`.

## Sostegno

Trade-free: niente tracciamento, niente pubblicità. Se ti è utile, puoi sostenere l’autore: https://buymeacoffee.com/echo.of.the.strings
