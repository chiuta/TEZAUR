# TEZAUR

[English](README.md) · **Română** · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versiune: 0.5.2 · Ultima actualizare: 2026-11-14

Un asistent de cercetare local, ancorat în citări, pentru scrierea unei teze. Un singur binar Rust.
Cărțile și articolele tale rămân pe calculatorul tău: nimic nu este trimis nicăieri decât dacă rulezi una dintre comenzile online opționale (vezi „Utilizarea rețelei”).
```
PDF / EPUB / DOCX / txt ──► chunk (page-tagged) ──► index (BM25 + optional embeddings)
   question or section title ──► retrieve ──► local model writes ONLY from the excerpts, with [n] citations
                                             └─► post-check flags invented citations, builds the reference list
```
## De ce regăsire, și nu „antrenare pe cărți”

Un laptop nu poate rafina (fine-tune) un model pe o bibliotecă, iar un model rafinat tot ar halucina
surse. Regăsirea oferă garanția opusă: fiecare afirmație trimite la un pasaj (fișier + pagină) pe care îl
poți deschide și verifica, iar modelul este obligat să spună ce *nu* conține biblioteca.

## Instalare din depozitele de pachete (construite și publicate de CI la fiecare push)

La fiecare push, aceste depozite sunt reconstruite pe GitHub Pages și se instalează din ele în containere curate înainte de publicare (vezi `.github/workflows/pages.yml`).

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
Semnare: depozitele sunt semnate GPG abia după ce mentenantul adaugă secretul `GPG_PRIVATE_KEY` (vezi `deploy/signing/README.md`); până atunci sunt nesemnate. Mecanismul este testat în CI cu o cheie de unică folosință. Arhivele de versiune și fișierele `.deb` poartă atestări de proveniență a build-ului Sigstore, fără chei: `gh attestation verify FILE -R chiuta/TEZAUR`. Indexul F-Droid este semnat cu o cheie de unică folosință generată la fiecare build, așa că Android îți va cere să reinstalezi aplicația când se schimbă.

## Tutoriale video

13 tutoriale scurte (aplicația web, CLI, tabele/ecuații din PDF, audit, MCP, API-ul serverului, Mesh, instalare), cu subtitrări în engleză, română, franceză, italiană, spaniolă, portugheză, germană și maghiară, sunt generate de `scripts/tutorials/make.py` (vezi `scripts/tutorials/README.md`). Videoclipurile se află în `assets/tutorials/` (un `.mp4` cu 8 piste de subtitrare selectabile + câte 8 `.vtt`) și rulează pe pagina de prezentare, tezaur.chiuta.ro (aplicația web instalabilă se află sub `/app/`). Ieșirea din terminal este reală; modelul din demonstrații este o simulare, iar traducerile sunt traduceri automate nerevizuite.

## Backend-uri de model

TEZAUR are nevoie de un server local de modele pentru răspunsuri (nu pentru `search`, `sources`, `ingest`).

| `--api`  | Server | Adresă implicită |
|----------|--------|------------------|
| `ollama` | Ollama | `http://localhost:11434` |
| `jan`    | [Jan](https://jan.ai) — Settings → Local API Server → Start Server | `http://127.0.0.1:1337/v1` |
| `openai` | LM Studio, llama.cpp `llama-server`, vLLM… (dă `--host`) | `http://127.0.0.1:8080/v1` |

```bash
tezaur --api jan models            # lists the model ids Jan serves; use one as --model
tezaur --api jan --model <id> ask "…"
```
Sau pune `api = jan` și `model = <id>` în `Data/settings/tezaur.conf` (portabil), ca să nu mai fie nevoie de opțiuni.
Cu Jan/LM Studio, fereastra de context este o setare a modelului de acolo, nu ceva ce TEZAUR poate cere.
Embeddings (`--embed-model`) funcționează doar dacă serverul oferă un endpoint de embeddings; dacă nu, ingest păstrează
documentele tale și căutarea după cuvinte-cheie, și spune asta.

## Configurare (exemplu Ollama)

```bash
cargo build --release                      # binary: target/release/tezaur
ollama pull llama3.1:8b                    # generation; bigger model = better and slower
ollama pull bge-m3                         # optional: multilingual embeddings (Romanian/English/French)
tezaur ingest ~/thesis/library --embed-model bge-m3
```
Intrări: **pdf, docx, epub, txt, md, html, tex**. Rularea din nou a `ingest` procesează doar fișierele noi sau modificate;
după actualizarea TEZAUR, rulează o dată `ingest --force` ca să recitești totul cu noua împărțire în fragmente și cu cititoarele noi.
Pasajele sunt tăiate, când e posibil, la limitele propozițiilor; EPUB-urile sunt citite în ordinea proprie de lectură a cărții.

## Comenzi

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
- `--only smith` (interactiv: `/only smith`) restrânge totul la documentele al căror titlu, cale sau
  citare se potrivește — sursele unui singur capitol pe rând, ca într-un spațiu de lucru.
- **BibTeX**: pune un export `.bib` din Zotero / Better BibTeX lângă index (portabil: oriunde în `Data/`).
  Un fișier este potrivit după cheia de citare = numele fișierului, sau după câmpul `file` al intrării. Citările devin atunci
  „Smith & Doe (2019), p. 3”, iar lista de referințe poartă detaliile complete. Nu este nevoie de reindexare.
- **Avertisment de potrivire slabă**: în modul doar cu cuvinte-cheie, TEZAUR avertizează când cel mai bun pasaj acoperă sub 40% din
  termenii interogării, în loc să lase modelul să improvizeze pe nimic.

Setări (opțiuni / variabile de mediu / `tezaur.conf`): `--api/TEZAUR_API`, `--host/TEZAUR_HOST` (Ollama și `OLLAMA_HOST`),
`--api-key/TEZAUR_API_KEY`, `--model/TEZAUR_MODEL`, `--embed-model/TEZAUR_EMBED_MODEL`,
`--num-ctx/TEZAUR_NUM_CTX` (Ollama), `--index/TEZAUR_INDEX`, `--only`, `--bib`.

Intrări bibliografice și interogări (toate offline, dacă nu se spune altfel):

- **Formate**: pe lângă `.bib`, sunt încărcate și fișierele care se termină în `.ris` și `.csl.json` din dosarul bibliografiei
  (`--bib file.json` încarcă explicit orice fișier CSL JSON). Tipurile de referințe sunt mapate prin CSL JSON;
  sunt citite `@string` din BibTeX, concatenarea cu `#` și moștenirea `crossref`. DOI, ISBN, ISSN și ORCID iD
  sunt extrase, iar cifrele lor de control sunt verificate; `--unify-duplicates` lasă intrările cu același DOI/ISBN
  să răspundă la cheile de citare ale celorlalte.
- **Fraze și proximitate** (opțional): cu `--phrases` (sau `TEZAUR_PHRASES=1`), `"exact phrase"` în `search`, `ask` și
  API potrivește cuvintele în ordine, iar `"a b"~5` la cel mult 5 cuvinte unul de altul; pasajele fără frază
  sunt eliminate, iar un `~k` malformat oprește interogarea cu o eroare. Fără opțiune, ghilimelele sunt semne de punctuație obișnuite,
  exact ca înainte, deci întrebările în limbaj natural care conțin ghilimele își păstrează potrivirile parțiale.
  Potrivirea ignoră diacriticele, majusculele și forma Unicode (memoria cache BM25 se reconstruiește o dată după actualizare).
- **Localizatori** în `audit`: `[@key, p. 3]`, `pp. 3-5`, `p. 12 ff.`, cifre romane, etichete `ch.`/`sec.`; un interval
  este verificat pe fiecare pagină pe care o numește (`page-out-of-range`, `bad-range`).
- **EPUB**: numerele de pagină tipărite (page-list / `epub:type="pagebreak"`) sunt folosite ca numere de pagină când cartea le are.
- **Chitanțe**: `ask --receipt` scrie `tezaur-receipt/1`, ca înainte. Setează `TEZAUR_RECEIPT_FORMAT=2` ca să scrii
  în schimb `tezaur-receipt/2` (CLI, API HTTP și MCP deopotrivă): rezumatul criptografic și HMAC-ul opțional (`TEZAUR_RECEIPT_KEY`)
  sunt atunci calculate peste forma canonică RFC 8785, deci reformatarea fișierului nu contează. Documentele sunt denumite prin SHA-256
  (înregistrat la ingestie într-un fișier nou `<index>.sha256` lângă index; fișierul index în sine este identic octet cu octet
  cu înainte, iar fișierul însoțitor este scris doar când se scrie un index, niciodată de comenzile numai-citire). Documentele deja aflate în
  un index construit fără el sunt marcate „identitate slabă” până la reingestia cu `--force`; un `ingest` simplu al
  fișierelor neschimbate nu îl adaugă. `PUT`/`GET /api/index` copiază doar fișierul index, deci o copie sincronizată nu are identități SHA-256.
  Ambele versiuni de chitanță se verifică, oricare format este selectat în prezent.
- **Politețea față de Crossref** (`refs --online`, `lit`): `--mailto you@example.org` (sau `TEZAUR_MAILTO`) pune adresa ta
  în User-Agent pentru „polite pool” al Crossref; cererile respectă antetele de rată ale serverului, fac backoff la 429, se opresc la 403,
  iar răspunsurile sunt păstrate pe disc (30 de zile pentru găsiri, 1 zi pentru ratări); `--refresh-online` ocolește memoria cache.

## Lanțul sursei (plicuri FFF, offline)

TEZAUR este al doilea pas al unui lanț care duce un document scanat la o referință citabilă fără rețea:
`legatorocr scan.png --ocr --author "Name, Given" --fff-year 1999 --fff-out doc.fff.json` -> `tezaur ingest --fff-in doc.fff.json` -> `tezaur bib --fff-out bib.fff.json` -> `veriga --fff-in bib.fff.json --fff-out checked.fff.json` -> AcademiSearch (Library -> Receive).

- `ingest --fff-in FILE` (repetabil) acceptă un plic de tip `document/ocr` (text pe pagină, autor, an, SHA-256 al scanării), `text/plain` sau `text/markdown`. Plicul este copiat în `fff-inbox/` lângă index, o intrare BibTeX `fff-<key>.bib` este scrisă lângă index (autor, an, titlu, `note = sha256:…`), iar paginile sunt indexate, deci citările `[@key, p. N]` funcționează și `tezaur audit` le poate verifica. Fișierele `.fff.json` simple dintr-un dosar dat lui `ingest` sunt indexate și ele.
- `bib --fff-out FILE` (sau `--to fff`) scrie bibliografia ca plic `citation/csl`, pentru VERIGA sau AcademiSearch.
- Limite: erorile OCR din text sunt indexate așa cum sunt; autorul/anul intrării sunt cele date pasului anterior, nu sunt verificate; formatul plicului este `docs/FFF-ENVELOPE.md` în github.com/chiuta/chiuta.github.io.

## Portabil (format PortableApps.com)

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
Rularea `tezaur.exe` fără nicio comandă deschide modul interactiv. Documentele din `Data/` sunt stocate după
cale relativă, deci dosarul poate fi mutat între unități și mașini. Nimic nu este scris în registry,
în `%APPDATA%` sau în profilul utilizatorului. Dacă folosești Jan în loc de Ollama, Jan își păstrează propriul dosar de date (poate fi
mutat din setările Jan); doar partea TEZAUR este portabilă.

## Versiunea HTML dintr-un singur fișier

`tezaur.html` este aceeași idee într-un singur fișier offline: deschide-l într-un browser, adaugă-ți fișierele, indică-l către Ollama / Jan /
LM Studio. Căutare BM25 (fără embeddings), ask / compare / gaps / summarize / outline / draft / flashcards, BibTeX,
verificarea citărilor, export Anki. Indexul trăiește în browser (IndexedDB). PDF-urile au nevoie de pdf.js, descărcat o dată dintr-un CDN.
Serverul de modele trebuie să permită accesul din browser (CORS): Ollama `OLLAMA_ORIGINS=*`, Jan → Local API Server → CORS.

### Alte moduri de a-l rula

Vezi `variants/README.md`: server MCP, extensie de browser (Chrome/Firefox), aplicații desktop Electron și Tauri, `.deb`, Docker,
YunoHost, pluginuri pentru DokuWiki / WordPress / MediaWiki / Vim / Neovim / Emacs / Sublime / Notepad++ / Obsidian / VS Code / Joplin / Logseq /
Zotero / LibreOffice / Google Docs / Calibre, boți Slack / Discord / WhatsApp / Telegram / Matrix / e-mail, clienți PyPI + npm, rețete systemd / Helm /
Raspberry Pi / Termux / NixOS și **Mesh** (schimb peer-to-peer între cercetători, `variants/MESH.md`) — cu o stare sinceră pentru fiecare.

### Android / telefon

- **`pwa/`**: aceeași pagină ca aplicație web instalabilă (manifest + service worker). Pune dosarul pe orice gazdă HTTPS
  (de exemplu YunoHost-ul tău) și folosește Chrome → „Install app”. Funcționează offline după prima vizită (pagina și citirea PDF-urilor; serverul de modele este separat, iar dictarea din browser descarcă modelul de pe internet când este folosită).
- **`android/`**: un mic proiect WebView care încorporează `tezaur.html` într-un APK. Nu este nevoie de niciun lanț de unelte local:
  GitHub → Actions → *android-apk* → Run workflow → descarcă `Tezaur-apk` (semnat de depanare, instalabil). Sau deschide
  `android/` în Android Studio. **Încă nu a fost construit și nici rulat** (SDK-ul nu poate fi descărcat în sandbox-ul meu).
  Serverul de modele trebuie să fie accesibil de pe telefon (Ollama / Jan pe PC-ul tău, aceeași rețea Wi-Fi) cu CORS activat.

## Verificări pe care nicio altă unealtă nu ți le oferă (și limitele lor)

- **`audit`** citește ciorna ta, găsește fiecare `[@key, p. 3]` în stil pandoc, caută *acea pagină* din *acea sursă* și raportează propozițiile pe care
  pagina nu le acoperă (`unsupported`, `weak`), o pagină mai probabilă (`wrong-page`), chei care nu se potrivesc cu niciun document, pagini dincolo de lungimea documentului,
  afirmații lungi fără citare și ghilimele al căror text nu este cuvânt cu cuvânt în sursă. Este un test de suprapunere de termeni: o parafrază care
  folosește alte cuvinte va obține un scor mic, iar o propoziție care repetă cuvintele potrivite cu sensul greșit va obține un scor mare. Îți spune unde să deschizi cartea.
- **`audit --semantic`** (v0.3) întreabă modelul tău local, pentru fiecare propoziție citată, dacă pasajul spune același lucru (SUPPORTS / PARTIAL / CONTRADICTS / UNRELATED).
  Poate salva o parafrază corectă și poate prinde o negație pe care suprapunerea de cuvinte o acceptă; fiecare verdict pe care îl schimbă este marcat `[model]` în notă.
  Este doar la fel de bun ca modelul pe care îl rulezi și a fost testat aici doar cu un înlocuitor scriptat (vezi REVIEW.md).
- **`refs`** este contabilitate: chei citate care nu există nici în `.bib`, nici în bibliotecă, intrări `.bib` și documente indexate pe care textul tău nu le citează niciodată.
  `refs --online`, `lit` și `oai` (recoltare OAI-PMH de la URL-ul depozitului pe care îl dai) sunt **singurele** comenzi care contactează internetul (OpenAlex, Crossref; `TEZAUR_MAILTO` pentru „polite pool” al lor;
  `TEZAUR_OPENALEX` / `TEZAUR_CROSSREF` pentru a indica o oglindă). Nu rulează niciodată implicit și afișează o notificare înainte de a trimite ceva.
- **Tabele și ecuații în PDF-uri** (v0.4): `ingest` păstrează un tabel împreună ca un singur pasaj Markdown (întâi legenda, antetul repetat dacă trebuie împărțit) și scrie
  ecuațiile afișate ca `[equation] $$ LaTeX $$ (eq. N)`; `tezaur tables FILE [--csv|--json|-o DIR]` și `tezaur equations FILE` le exportă. Paginile pe două coloane sunt citite coloană cu coloană.
  Este o **euristică peste stratul de text al poppler** (are nevoie de `pdftotext`; fără el, ingest revine la text simplu): coloanele sunt recuperate din goluri de 2+ spații, deci tabelele foarte strânse,
  celulele îmbinate/pe mai multe rânduri, tabelele care sunt imagini, fracțiile suprapuse, matricele și indicii/exponenții (aplatizați: `pi ti`) nu sunt reconstruite. Nu înlocuiește un model OCR precum Nougat sau MinerU.
- **`eval`** transformă „căutarea pare în regulă” în cifre pe propriile tale întrebări.
- Indexul de căutare păstrează o memorie cache BM25 lângă fișierul index (`<index>.bm25`, reconstruită automat când biblioteca se schimbă): o căutare CLI peste 6 M de cuvinte a scăzut de la ~4,5 s la ~0,2 s (remăsurat la 2026-10-30 pe 6 M de cuvinte sintetice, în acest sandbox; prima căutare după un ingest plătește cei ~4,5 s de construire a memoriei cache).
- **`coverage`** și **`novelty`** compară un plan sau un text nou cu ceea ce conține de fapt biblioteca ta.
- **Chitanțe** (`ask --receipt`): SHA-256 peste întrebare, model, răspuns și fiecare extras; `receipt` îl verifică ulterior, precum și faptul că acele extrase încă există neschimbate
  în index. Cu `TEZAUR_RECEIPT_KEY` setat, poartă și un HMAC. Aceasta este dovadă de manipulare, nu semnătură de identitate și nici dovadă că un răspuns este adevărat.
  `declare` transformă chitanțele într-o declarație simplă de utilizare a IA (în română sau engleză) cu o linie pe care trebuie s-o confirmi tu însuți.
- **Notebook-uri**: `%load_ext tezaur_client.magics` în Jupyter oferă `%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage`; `python -m tezaur_client.notebook thesis.ipynb` auditează proza unui notebook.

Vezi [COMPETITION.md](COMPETITION.md) pentru comparația cu uneltele de azi și [REVIEW.md](REVIEW.md) pentru o evaluare măsurată, fără menajamente, a ceea ce este și ce nu este dovedit (cifre de acuratețe ale auditului, scară, constatări de securitate).

## Limite sincere

- **Nu este un scriitor de teze.** Redactează din sursele tale; argumentul, structura și verificarea îți aparțin.
- Verificarea citărilor este mecanică: prinde numere de extrase inexistente și paragrafe lungi necitate,
  dar nu poate spune dacă un pasaj citat susține cu adevărat o propoziție. **Deschide pagina.**
- `summarize` poartă referințele de pagină prin mai multe apeluri de model; TEZAUR verifică dacă ele există în document,
  nu dacă fiecare este atașată propoziției potrivite. Verifică prin sondaj.
- `gaps` vede doar extrasele regăsite: raportează ce lasă deschis acele pasaje, nu tot ce lipsește.
- Modelele mici parafrazează aproximativ. Modelele mai mari (și rulările mai lente) sunt vizibil mai fidele.
- PDF-urile scanate au nevoie întâi de OCR (de exemplu `ocrmypdf`); sunt raportate ca „fără strat de text”.
- EPUB/DOCX nu au numere de pagină, deci citările lor nu poartă pagină. Capitolele EPUB sunt citite în ordinea proprie a cărții (spine).
- Fragmentele sunt ferestre de ~200 de cuvinte (40 suprapuse) tăiate la sfârșitul propozițiilor, fără a ține cont de capitole. Tabelele și ecuațiile din PDF-uri sunt recuperate doar euristic (vezi mai sus).
- Calitatea regăsirii este plafonul: dacă `search` nu scoate la suprafață pasajul potrivit, `ask` nu îl poate folosi.

- **Flashcards** păstrează un card doar dacă răspunsul lui citează un extras existent; restul sunt eliminate și numărate.
  Fișierul `.tsv` are liniile de antet `#separator:tab` / `#html:false` și două coloane (față, spate + sursă); nu am putut
  testa importul în Anki însuși.
- **Tipurile de ciornă** schimbă cerința de redactare, nu regulile: fiecare tip tot citează extrase, iar `introduction`
  lasă scopurile tale proprii ca un substituent `[AUTHOR: …]`, în loc să le inventeze.

## De unde au venit ideile

Examinate: AnythingLLM, GPT4All LocalDocs, Kotaemon, Khoj, NotebookLM, PrivateGPT, asistenți bazați pe Zotero și
distribuția Linux pentru studenți UberStudent (acum abandonată). De la UberStudent: un meniu organizat pe competențe academice
(citește / cercetează / scrie / studiază — aspectul `/help`), ajutoare de studiu construite în jurul Anki, șabloane pentru lucrări academice
(`draft --kind`) și un ghid pentru prima rulare (`doctor`, indiciul „pasul următor”). UberStudent rula și direct de pe un stick USB,
ceea ce este scopul aspectului portabil.

## CI

- `.github/workflows/ci.yml` — la fiecare push/PR: build, `cargo test`, construiește toate pluginurile și rulează întreaga suită de integrare (`variants/tests/run-all.sh`, cu poppler și pdflatex instalate ca să ruleze testele PDF); construiește și testează și pe Windows și macOS.
- `.github/workflows/release.yml` — manual sau la un tag `v*`: CLI (3 sisteme de operare), .deb, .rpm, Electron, Tauri, instalator Windows, extensii, Obsidian/VS Code/Office, Capacitor APK și fiecare plugin/integrare (`variants/build-plugins.sh`); la un tag, un job final atașează toate artefactele și un `SHA256SUMS.txt` la versiunea GitHub.
- `portable.yml`, `android.yml`, `docker.yml`, `pages.yml` — dosarul Windows PortableApps, Android, imaginea Docker, PWA pe GitHub Pages.

**Niciunul dintre aceste workflow-uri nu a rulat încă pe GitHub** (doar pașii lor au fost exersați local pe Linux); așteaptă-te să trebuiască să repari o cale sau un pachet lipsă la prima rulare, mai ales pentru Windows, macOS, Electron, Tauri, iOS și Android.

## Deliberat nerealizat

- **Salvarea răspunsurilor modelului înapoi în bibliotecă drept surse**: următorul răspuns ar putea atunci cita
  propriul text anterior al modelului ca și cum ar fi dovadă. Salvează ciornele cu `-o` și ingerează-le doar dacă accepți asta.
- **Servicii cloud de embeddings / reranking**: contrar scopului unei unelte locale. Un reranker local este o posibilă
  adăugire viitoare.

## Teste

`cargo test` (61 de teste, inclusiv vectori SHA-256/HMAC și logica audit/chitanțe) și `variants/tests/run-all.sh` — tokenizator/diacritice, BM25, fuziunea clasamentelor, filtrul de documente, împărțirea în fragmente, citirea DOCX/EPUB, parsarea
și potrivirea BibTeX, parsarea citărilor și rescrierea paginilor, parsarea REPL, aspectul portabil și clientul de model
pentru ambele protocoale (Ollama NDJSON, OpenAI SSE) față de servere false în proces.

## Licență

MIT © 2026 Alexio (https://trom.tf) — vezi [LICENSE](LICENSE).

## Benchmark de audit al citărilor și verificarea referințelor

`python3 variants/tests/audit_bench.py target/release/tezaur --check` măsoară `audit`-ul mecanic pe 24 de pasaje inventate x 7 tipuri de afirmații = 168 de verificări (cuvânt cu cuvânt, parafrază, cifră scrisă altfel, cifră sau cuvânt schimbat, negație, sursă greșită, fără legătură). Este **sintetic și construit din șabloane**: arată ce poate și ce nu poate vedea verificarea lexicală, nu acuratețea pe teze reale. Ultima rulare:

| Tip de afirmație | Așteptat | Corect | Rată |
|---|---|---|---|
| verbatim | ar trebui acceptat | 24/24 | 100 % |
| paraphrase | trimis unui om (nepunctat) | 24/24 | 100 % |
| figure-form | ar trebui acceptat | 22/24 | 92 % |
| figure-swap | ar trebui semnalat | 19/19 | 100 % |
| negation | ar trebui semnalat | 1/24 | 4 % |
| wrong-source | ar trebui semnalat | 24/24 | 100 % |
| unrelated | ar trebui semnalat | 24/24 | 100 % |
| word-swap | ar trebui semnalat | 2/5 | 40 % |

În total: recall pentru „semnalat când trebuie” 73 % (70/96); precizie 97 % (70/72); alarme false pe afirmații corecte 2/48.

Cum se citește: sursele greșite, propozițiile fără legătură și cifrele schimbate sunt prinse; negațiile și cuvintele singulare schimbate **nu** sunt (folosește `audit --semantic` cu un model local, a cărui calitate nu este testată cu un model real); parafrazele sunt trimise unui om prin construcție.

`tezaur audit draft.md --online [--fail]` verifică în plus că fiecare referință citată există și nu este retrasă (OpenAlex/Crossref). Este opțional, afișează la stderr ce trimite și trimite doar DOI-ul/titlul intrărilor `.bib` citate. Testat cu servicii simulate; serviciile reale nu au putut fi accesate din sandbox-ul de dezvoltare.

## Utilizarea rețelei și unelte externe

pdf.js 3.11.174 (Apache-2.0) este inclus local în PWA (`pwa/vendor/pdfjs`, păstrat în cache de service worker-ul ei), în extensia de browser (`variants/build-extension.sh` îl copiază, iar `build-web.py --local-pdfjs` îndreaptă pagina spre el) și în varianta YunoHost; `tezaur.html` dintr-un singur fișier și celelalte învelișuri construite cu `build-web.py` îl încarcă de la jsDelivr cu un hash Subresource Integrity (worker-ul PDF nu este verificat pentru integritate). transformers.js (jsDelivr, import dinamic, fără SRI posibil) și modelul Whisper tiny (Hugging Face) sunt încă descărcate de pe internet; boții transmit mesajele prin servicii terțe. „Local” se aplică, așadar, documentelor tale și indexului de regăsire, nu fiecărei variante în orice moment. Funcțiile PDF și OCR au nevoie de `pdftotext`, `pdftoppm` și `tesseract` instalate. Modelele recomandate (`llama3.1:8b`, `bge-m3`) nu sunt incluse și poartă propriile licențe (Llama 3.1 are propria licență de comunitate, cu restricții de utilizare). Proveniență: `docs/clean-room/PROVENANCE.md`.

## Sprijin

Trade-free: fără urmărire, fără reclame. Dacă îți este de folos, îl poți sprijini pe autor: https://buymeacoffee.com/echo.of.the.strings
