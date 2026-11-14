# TEZAUR

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · **Magyar**

Verzió: 0.5.2 · Utolsó frissítés: 2026-11-14

Helyi, idézetekre épülő kutatási asszisztens szakdolgozat vagy disszertáció írásához. Egyetlen Rust bináris.
A könyveid és cikkeid a gépeden maradnak: semmi nem megy sehová, hacsak nem futtatod valamelyik opcionális online parancsot (lásd „Hálózathasználat”).
```
PDF / EPUB / DOCX / txt ──► chunk (page-tagged) ──► index (BM25 + optional embeddings)
   question or section title ──► retrieve ──► local model writes ONLY from the excerpts, with [n] citations
                                             └─► post-check flags invented citations, builds the reference list
```
## Miért keresés (retrieval), és nem „tanítás a könyveken”

Egy laptop nem tud egy modellt egy könyvtáron finomhangolni (fine-tuning), és a finomhangolt modell is
forrásokat találna ki. A keresés az ellenkező garanciát adja: minden állítás egy szövegrészre (fájl + oldal) mutat, amelyet
megnyithatsz és ellenőrizhetsz, a modellt pedig arra kényszeríti, hogy kimondja, mit *nem* tartalmaz a könyvtár.

## Telepítés a csomagtárolókból (a CI építi és teszi közzé minden push után)

Minden push után ezek a tárolók újraépülnek a GitHub Pagesen, és a közzététel előtt tiszta konténerekben telepítik belőlük a csomagot (lásd `.github/workflows/pages.yml`).

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
Aláírás: a tárolók csak akkor lesznek GPG-aláírva, ha a karbantartó hozzáadja a `GPG_PRIVATE_KEY` titkot (lásd `deploy/signing/README.md`); addig aláíratlanok. A mechanizmust a CI egy eldobható kulccsal teszteli. A kiadási archívumok és a `.deb` fájlok kulcs nélküli Sigstore build-eredet-igazolást hordoznak: `gh attestation verify FILE -R chiuta/TEZAUR`. Az F-Droid index buildenként generált eldobható kulccsal van aláírva, ezért az Android újratelepítést kér, ha az megváltozik.

## Videós útmutatók

13 rövid útmutató (webalkalmazás, CLI, PDF-táblázatok/egyenletek, audit, MCP, szerver-API, Mesh, telepítés) angol, román, francia, olasz, spanyol, portugál, német és magyar felirattal készül a `scripts/tutorials/make.py` szkripttel (lásd `scripts/tutorials/README.md`). Maguk a videók az `assets/tutorials/` mappában vannak (egy `.mp4` 8 választható feliratsávval + 8 `.vtt`), és a bemutatóoldalon, a tezaur.chiuta.ro címen játszhatók le (a telepíthető webalkalmazás az `/app/` alatt található). A terminálkimenet valódi; a bemutatókban a modell szimulált, a fordítások pedig lektorálatlan gépi fordítások.

## Modell-háttérrendszerek

A TEZAUR-nak a válaszokhoz helyi modellszerverre van szüksége (a `search`, `sources`, `ingest` parancsokhoz nem).

| `--api`  | Szerver | Alapértelmezett cím |
|----------|---------|---------------------|
| `ollama` | Ollama | `http://localhost:11434` |
| `jan`    | [Jan](https://jan.ai) — Settings → Local API Server → Start Server | `http://127.0.0.1:1337/v1` |
| `openai` | LM Studio, llama.cpp `llama-server`, vLLM… (add meg a `--host` értékét) | `http://127.0.0.1:8080/v1` |

```bash
tezaur --api jan models            # lists the model ids Jan serves; use one as --model
tezaur --api jan --model <id> ask "…"
```
Vagy írd be az `api = jan` és `model = <id>` sorokat a `Data/settings/tezaur.conf` fájlba (hordozható), és nincs szükség kapcsolókra.
Janhoz/LM Studióhoz a kontextusablak az ottani modell beállítása, nem olyasmi, amit a TEZAUR kérhetne.
Az embeddingek (`--embed-model`) csak akkor működnek, ha a szerver kínál embedding végpontot; ha nem, az ingest megtartja
a dokumentumaidat és a kulcsszavas keresést, és ezt jelzi is.

## Beállítás (Ollama példa)

```bash
cargo build --release                      # binary: target/release/tezaur
ollama pull llama3.1:8b                    # generation; bigger model = better and slower
ollama pull bge-m3                         # optional: multilingual embeddings (Romanian/English/French)
tezaur ingest ~/thesis/library --embed-model bge-m3
```
Bemenetek: **pdf, docx, epub, txt, md, html, tex**. Az `ingest` újrafuttatása csak az új vagy módosult fájlokat dolgozza fel;
a TEZAUR frissítése után futtasd egyszer az `ingest --force` parancsot, hogy minden újraolvasódjon az újabb darabolással és olvasókkal.
A szövegrészeket lehetőség szerint mondathatárokon vágja; az EPUB-okat a könyv saját olvasási sorrendjében olvassa.

## Parancsok

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
- Az `--only smith` (interaktívan: `/only smith`) mindent azokra a dokumentumokra szűkít, amelyeknek a címe, útvonala vagy
  hivatkozása egyezik — egyszerre egy fejezet forrásaira, mint egy munkaterületen.
- **BibTeX**: tegyél egy Zotero / Better BibTeX `.bib` exportot az index mellé (hordozható módban: bárhová a `Data/` alatt).
  A fájlt a hivatkozási kulcs = fájlnév alapján, vagy a bejegyzés `file` mezője alapján párosítja. A hivatkozások ekkor
  „Smith & Doe (2019), 3. o.” alakban jelennek meg, és az irodalomjegyzék a teljes adatokat tartalmazza. Újraindexelésre nincs szükség.
- **Gyenge találat figyelmeztetése**: csak kulcsszavas módban a TEZAUR figyelmeztet, ha a legjobb szövegrész a lekérdezés
  kifejezéseinek 40%-ánál kevesebbet fed le, ahelyett, hogy hagyná a modellt a semmire rögtönözni.

Beállítások (kapcsolók / környezeti változók / `tezaur.conf`): `--api/TEZAUR_API`, `--host/TEZAUR_HOST` (Ollamánál az `OLLAMA_HOST` is),
`--api-key/TEZAUR_API_KEY`, `--model/TEZAUR_MODEL`, `--embed-model/TEZAUR_EMBED_MODEL`,
`--num-ctx/TEZAUR_NUM_CTX` (Ollama), `--index/TEZAUR_INDEX`, `--only`, `--bib`.

Irodalomjegyzék-bemenetek és lekérdezések (mind offline, ha nincs másként jelezve):

- **Formátumok**: a `.bib` mellett a bibliográfia mappájában a `.ris` és `.csl.json` végű fájlok is betöltődnek
  (a `--bib file.json` kifejezetten bármely CSL JSON fájlt betölt). A hivatkozástípusok CSL JSON-on keresztül képeződnek le;
  a BibTeX `@string`, a `#` összefűzés és a `crossref` öröklés is olvasódik. A DOI-k, ISBN-ek, ISSN-ek és ORCID iD-k
  kinyerésre kerülnek, ellenőrző számjegyeiket ellenőrzi; az `--unify-duplicates` lehetővé teszi, hogy az azonos DOI/ISBN-nel rendelkező bejegyzések
  egymás hivatkozási kulcsaira is válaszoljanak.
- **Kifejezések és közelség** (opcionális): a `--phrases` kapcsolóval (vagy `TEZAUR_PHRASES=1`) az `"exact phrase"` a `search`, `ask` és
  az API-ban a szavakat sorrendben keresi, az `"a b"~5` pedig egymástól legfeljebb 5 szónyira; a kifejezés nélküli szövegrészek
  kiesnek, a hibás `~k` pedig hibával állítja le a lekérdezést. A kapcsoló nélkül az idézőjelek közönséges
  írásjelek, pontosan mint korábban, így az idézőjelet tartalmazó természetes nyelvű kérdések megtartják részleges találataikat.
  Az egyeztetés ékezet-, kis- és nagybetű- és Unicode-forma-független (a BM25-gyorsítótár a frissítés után egyszer újraépül).
- **Helymeghatározók** az `audit` parancsban: `[@key, p. 3]`, `pp. 3-5`, `p. 12 ff.`, római számok, `ch.`/`sec.` címkék; egy tartományt
  minden megnevezett oldalon ellenőriz (`page-out-of-range`, `bad-range`).
- **EPUB**: a nyomtatott oldalszámokat (page-list / `epub:type="pagebreak"`) oldalszámként használja, ha a könyv rendelkezik velük.
- **Nyugták (receipt)**: az `ask --receipt` a korábbiak szerint `tezaur-receipt/1` formátumot ír. Állítsd a `TEZAUR_RECEIPT_FORMAT=2` értéket, hogy
  helyette `tezaur-receipt/2` készüljön (CLI, HTTP API és MCP egyaránt): az összefoglaló és az opcionális HMAC (`TEZAUR_RECEIPT_KEY`)
  ilyenkor az RFC 8785 szerinti kanonikus alakra készül, így a fájl átformázása nem számít. A dokumentumokat SHA-256 azonosítja
  (a betöltéskor az index melletti új `<index>.sha256` fájlban rögzítve; maga az indexfájl bájtra azonos
  a korábbival, a kísérőfájl pedig csak index írásakor készül, soha nem csak olvasó parancsoktól). Az olyan indexben már meglévő dokumentumok,
  amely nélküle épült, „gyenge azonosság” jelölést kapnak, amíg `--force` kapcsolóval újra nem töltöd be őket; a változatlan
  fájlok egyszerű `ingest` parancsa nem adja hozzá. A `PUT`/`GET /api/index` csak az indexfájlt másolja, így a szinkronizált másolatnak nincs SHA-256 azonossága.
  Mindkét nyugtaverzió ellenőrizhető, bármelyik formátum van is éppen kiválasztva.
- **Udvariasság a Crossrefhez** (`refs --online`, `lit`): a `--mailto you@example.org` (vagy `TEZAUR_MAILTO`) beírja a címedet
  a User-Agentbe a Crossref „polite pool” számára; a kérések követik a szerver sebességfejléceit, 429-nél lassítanak, 403-nál leállnak,
  a válaszokat lemezen gyorsítótárazza (a találatokat 30 napig, a hiányokat 1 napig); a `--refresh-online` megkerüli a gyorsítótárat.

## Forrásláncolat (FFF-borítékok, offline)

A TEZAUR egy lánc második lépése, amely egy beolvasott dokumentumból hálózat nélkül idézhető hivatkozást készít:
`legatorocr scan.png --ocr --author "Name, Given" --fff-year 1999 --fff-out doc.fff.json` -> `tezaur ingest --fff-in doc.fff.json` -> `tezaur bib --fff-out bib.fff.json` -> `veriga --fff-in bib.fff.json --fff-out checked.fff.json` -> AcademiSearch (Library -> Receive).

- Az `ingest --fff-in FILE` (ismételhető) `document/ocr` típusú borítékot (oldalankénti szöveg, szerző, év, a beolvasás SHA-256 értéke), `text/plain` vagy `text/markdown` típust fogad. A borítékot az index melletti `fff-inbox/` mappába másolja, az index mellé egy `fff-<key>.bib` BibTeX-bejegyzést ír (szerző, év, cím, `note = sha256:…`), az oldalakat pedig indexeli, így a `[@key, p. N]` hivatkozások működnek, és a `tezaur audit` ellenőrizni tudja őket. Az `ingest`-nek átadott mappában lévő egyszerű `.fff.json` fájlok is indexelődnek.
- A `bib --fff-out FILE` (vagy `--to fff`) a bibliográfiát `citation/csl` borítékként írja ki, a VERIGA vagy az AcademiSearch számára.
- Korlátok: a szövegben lévő OCR-hibákat úgy indexeli, ahogy vannak; a bejegyzés szerzője/éve az, amit az előző lépés kapott, nem ellenőrzött; a boríték formátuma a `docs/FFF-ENVELOPE.md` fájlban található a github.com/chiuta/chiuta.github.io tárolóban.

## Hordozható (PortableApps.com formátum)

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
A `tezaur.exe` parancs nélküli futtatása az interaktív módot nyitja meg. A `Data/` mappán belüli dokumentumokat
relatív útvonallal tárolja, így a mappa mozgatható meghajtók és gépek között. Semmi nem íródik a rendszerleíró adatbázisba,
az `%APPDATA%` mappába vagy a felhasználói profilba. Ha Ollama helyett Jant használsz, a Jan a saját adatmappáját tartja (ez
a Jan beállításaiban áthelyezhető); csak a TEZAUR oldala hordozható.

## Egyfájlos HTML-változat

A `tezaur.html` ugyanez az ötlet egyetlen offline fájlban: nyisd meg böngészőben, add hozzá a fájljaidat, irányítsd Ollamára / Janra /
LM Studióra. BM25-keresés (embedding nélkül), ask / compare / gaps / summarize / outline / draft / flashcards, BibTeX,
hivatkozás-ellenőrzés, Anki-export. Az index a böngészőben él (IndexedDB). A PDF-ekhez pdf.js kell, amelyet egyszer tölt le egy CDN-ről.
A modellszervernek engedélyeznie kell a böngészős hozzáférést (CORS): Ollama `OLLAMA_ORIGINS=*`, Jan → Local API Server → CORS.

### További futtatási módok

Lásd `variants/README.md`: MCP-szerver, böngészőbővítmény (Chrome/Firefox), Electron és Tauri asztali alkalmazások, `.deb`, Docker,
YunoHost, bővítmények DokuWiki / WordPress / MediaWiki / Vim / Neovim / Emacs / Sublime / Notepad++ / Obsidian / VS Code / Joplin / Logseq /
Zotero / LibreOffice / Google Docs / Calibre programokhoz, Slack / Discord / WhatsApp / Telegram / Matrix / e-mail botok, PyPI + npm kliensek, systemd / Helm /
Raspberry Pi / Termux / NixOS receptek, és a **Mesh** (kutatók közötti peer-to-peer csere, `variants/MESH.md`) — mindegyikhez őszinte állapotleírással.

### Android / telefon

- **`pwa/`**: ugyanez az oldal telepíthető webalkalmazásként (manifest + service worker). Tedd a mappát bármely HTTPS-tárhelyre
  (például a YunoHostodra), és használd a Chrome → „Install app” menüpontot. Az első látogatás után offline is működik (az oldal és a PDF-olvasás; a modellszerver különálló, a böngészős diktálás pedig használatkor az internetről tölti le a modelljét).
- **`android/`**: egy kis WebView-projekt, amely a `tezaur.html` fájlt APK-ba csomagolja. Helyi eszközlánc nem szükséges:
  GitHub → Actions → *android-apk* → Run workflow → töltsd le a `Tezaur-apk` fájlt (hibakereső aláírású, telepíthető). Vagy nyisd meg az
  `android/` mappát az Android Studióban. **Még nem építettem és nem futtattam** (az SDK nem tölthető le a homokozómban).
  A modellszervernek elérhetőnek kell lennie a telefonról (Ollama / Jan a számítógépeden, ugyanazon a Wi-Fi-n), bekapcsolt CORS-szal.

## Ellenőrzések, amelyeket más eszköz nem ad (és korlátaik)

- Az **`audit`** elolvassa a vázlatodat, megtalál minden pandoc-stílusú `[@key, p. 3]` hivatkozást, megkeresi *azt az oldalt* *abból a forrásból*, és jelzi azokat a mondatokat, amelyeket az
  oldal nem fed le (`unsupported`, `weak`), a valószínűbb oldalt (`wrong-page`), a semelyik dokumentumhoz sem illő kulcsokat, a dokumentum hosszán túli oldalakat,
  a hivatkozás nélküli hosszú állításokat, és azokat az idézőjeles részeket, amelyek szövege nem szó szerint szerepel a forrásban. Ez kifejezés-átfedési teszt: az a parafrázis, amely
  más szavakat használ, alacsony pontot kap, az a mondat pedig, amely a megfelelő szavakat ismétli rossz jelentéssel, magasat. Megmondja, hol nyisd ki a könyvet.
- Az **`audit --semantic`** (v0.3) minden idézett mondatnál megkérdezi a helyi modelledet, hogy a szövegrész ugyanazt mondja-e (SUPPORTS / PARTIAL / CONTRADICTS / UNRELATED).
  Megmenthet egy helyes parafrázist, és elkaphat egy tagadást, amelyet a szóátfedés elfogad; minden általa módosított ítélet `[model]` jelölést kap a megjegyzésben.
  Csak annyira jó, mint a futtatott modell, és itt csak egy szkriptelt helyettesítővel próbáltuk ki (lásd REVIEW.md).
- A **`refs`** könyvelés: idézett kulcsok, amelyek sem a `.bib`-ben, sem a könyvtárban nem léteznek, valamint `.bib` bejegyzések és indexelt dokumentumok, amelyeket a szöveged soha nem idéz.
  A `refs --online`, a `lit` és az `oai` (az általad megadott tároló URL-jének OAI-PMH-betakarítása) az **egyetlen** parancsok, amelyek az internetet érintik (OpenAlex, Crossref; `TEZAUR_MAILTO` az ő „polite pool”-jukhoz;
  `TEZAUR_OPENALEX` / `TEZAUR_CROSSREF` tükörre mutatáshoz). Soha nem futnak implicit módon, és bármi elküldése előtt értesítést írnak ki.
- **Táblázatok és egyenletek PDF-ekben** (v0.4): az `ingest` egy táblázatot egyetlen Markdown-szövegrészként tart egyben (előbb a felirat, a fejléc megismételve, ha kettéosztandó), és a
  kiemelt egyenleteket `[equation] $$ LaTeX $$ (eq. N)` alakban írja; a `tezaur tables FILE [--csv|--json|-o DIR]` és a `tezaur equations FILE` exportálja őket. A kétoszlopos oldalakat oszloponként olvassa.
  Ez a **poppler szövegrétege fölötti heurisztika** (`pdftotext` kell hozzá; nélküle az ingest egyszerű szövegre vált): az oszlopokat 2+ szóközös hézagokból állítja vissza, ezért a nagyon szoros táblázatokat,
  az összevont/többsoros cellákat, a képként megjelenő táblázatokat, a többszintes törteket, a mátrixokat és az alsó/felső indexeket (lapítva: `pi ti`) nem rekonstruálja. Nem helyettesíti az olyan OCR-modelleket, mint a Nougat vagy a MinerU.
- Az **`eval`** a „a keresés jónak tűnik” érzetet számokká alakítja a saját kérdéseiden.
- A keresési index az indexfájl mellett BM25-gyorsítótárat tart (`<index>.bm25`, a könyvtár változásakor automatikusan újraépül): egy 6 M szavas CLI-keresés ~4,5 s-ról ~0,2 s-ra gyorsult (2026-10-30-án újramérve 6 M szintetikus szón, ebben a homokozóban; az ingest utáni első keresés fizeti a ~4,5 s gyorsítótár-építést).
- A **`coverage`** és a **`novelty`** egy vázlatot vagy új szöveget hasonlít össze azzal, amit a könyvtárad valójában tartalmaz.
- **Nyugták** (`ask --receipt`): SHA-256 a kérdésen, a modellen, a válaszon és minden kivonaton; a `receipt` később ellenőrzi, és azt is, hogy ezek a kivonatok még változatlanul léteznek
  az indexben. Beállított `TEZAUR_RECEIPT_KEY` mellett HMAC-ot is hordoznak. Ez módosítás-bizonyíték, nem személyazonossági aláírás, és nem bizonyítja, hogy egy válasz igaz.
  A `declare` a nyugtákból egyszerű MI-használati nyilatkozatot készít (románul vagy angolul), egy sorral, amelyet neked magadnak kell megerősítened.
- **Notebookok**: a `%load_ext tezaur_client.magics` Jupyterben a `%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage` varázsparancsokat adja; a `python -m tezaur_client.notebook thesis.ipynb` egy notebook szövegét auditálja.

A mai eszközökhöz való viszonyáért lásd a [COMPETITION.md](COMPETITION.md) fájlt, a bizonyított és nem bizonyított dolgok mért, szépítés nélküli értékeléséért (az audit pontossági számai, skálázás, biztonsági megállapítások) pedig a [REVIEW.md](REVIEW.md) fájlt.

## Őszinte korlátok

- **Nem szakdolgozatíró.** A forrásaidból vázol; az érvelés, a szerkezet és az ellenőrzés a tiéd.
- A hivatkozás-ellenőrzés mechanikus: elkapja a nem létező kivonatszámokat és a hivatkozás nélküli hosszú bekezdéseket,
  de nem tudja megmondani, hogy egy idézett szövegrész valóban alátámaszt-e egy mondatot. **Nyisd meg az oldalt.**
- A `summarize` az oldalhivatkozásokat több modellhíváson át viszi; a TEZAUR azt ellenőrzi, hogy léteznek-e a dokumentumban,
  azt nem, hogy mindegyik a megfelelő mondathoz tartozik-e. Ellenőrizd mintavétellel.
- A `gaps` csak a megtalált kivonatokat látja: azt jelzi, amit ezek a szövegrészek nyitva hagynak, nem mindent, ami hiányzik.
- A kis modellek lazán parafrazálnak. A nagyobb modellek (és a lassabb futások) érezhetően hűségesebbek.
- A beolvasott PDF-ekhez előbb OCR kell (például `ocrmypdf`); „nincs szövegréteg” jelzést kapnak.
- Az EPUB/DOCX fájloknak nincs oldalszámuk, így hivatkozásaik sem hordoznak oldalt. Az EPUB-fejezeteket a könyv saját gerincsorrendjében (spine) olvassa.
- A darabok ~200 szavas ablakok (40 átfedéssel), mondatvégeken vágva, fejezetekre nem figyelve. A PDF-ek táblázatait és egyenleteit csak heurisztikusan állítja vissza (lásd fent).
- A keresés minősége a plafon: ha a `search` nem hozza felszínre a megfelelő szövegrészt, az `ask` nem tudja használni.

- A **Flashcards** csak akkor tart meg egy kártyát, ha a válasza létező kivonatra hivatkozik; a többit eldobja és megszámolja.
  A `.tsv` fájl `#separator:tab` / `#html:false` fejlécsorokat és két oszlopot tartalmaz (előlap, hátlap + forrás); az importálást magában az Ankiban nem
  tudtam kipróbálni.
- A **vázlattípusok** az írási megbízást változtatják, nem a szabályokat: minden típus továbbra is kivonatokra hivatkozik, az `introduction` pedig
  a saját céljaidat `[AUTHOR: …]` helykitöltőként hagyja, ahelyett hogy kitalálná őket.

## Honnan jöttek az ötletek

Átvizsgálva: AnythingLLM, GPT4All LocalDocs, Kotaemon, Khoj, NotebookLM, PrivateGPT, Zotero-alapú asszisztensek, valamint a
(mára felhagyott) UberStudent diák-Linux-disztribúció. Az UberStudentből: tudományos készségek szerint szervezett menü
(olvasás / kutatás / írás / tanulás — a `/help` elrendezése), az Anki köré épített tanulássegítők, tudományos munkák sablonjai
(`draft --kind`), valamint első indítási útmutató (`doctor`, a „következő lépés” tipp). Az UberStudent élő rendszerként USB-pendrive-ról is futott,
és a hordozható elrendezés erre való.

## CI

- `.github/workflows/ci.yml` — minden push/PR esetén: build, `cargo test`, az összes bővítmény építése és a teljes integrációs csomag futtatása (`variants/tests/run-all.sh`, telepített popplerrel és pdflatexszel, hogy a PDF-tesztek is lefussanak); Windowson és macOS-en is épít és tesztel.
- `.github/workflows/release.yml` — kézzel vagy `v*` címkére: CLI (3 operációs rendszer), .deb, .rpm, Electron, Tauri, Windows-telepítő, bővítmények, Obsidian/VS Code/Office, Capacitor APK, valamint minden bővítmény/integráció (`variants/build-plugins.sh`); címke esetén egy záró feladat az összes artefaktumot és egy `SHA256SUMS.txt` fájlt csatol a GitHub-kiadáshoz.
- `portable.yml`, `android.yml`, `docker.yml`, `pages.yml` — Windows PortableApps mappa, Android, Docker-kép, PWA a GitHub Pagesen.

**Ezek közül egyik munkafolyamat sem futott még a GitHubon** (csak a lépéseiket próbáltuk ki helyben, Linuxon); számíts rá, hogy az első futtatáskor ki kell javítanod egy útvonalat vagy egy hiányzó csomagot, különösen Windows, macOS, Electron, Tauri, iOS és Android esetén.

## Szándékosan nem épített

- **A modellválaszok visszamentése a könyvtárba forrásként**: a következő válasz ilyenkor a modell saját korábbi
  szövegére hivatkozhatna bizonyítékként. A vázlatokat mentsd a `-o` kapcsolóval, és csak akkor töltsd be őket, ha ezt vállalod.
- **Felhős embedding / újrarangsoroló szolgáltatások**: ellentétesek egy helyi eszköz lényegével. Egy helyi újrarangsoroló lehetséges
  jövőbeli kiegészítés.

## Tesztek

`cargo test` (61 teszt, köztük SHA-256/HMAC tesztvektorok és az audit-/nyugtalogika) és `variants/tests/run-all.sh` — tokenizáló/diakritikus jelek, BM25, rangsor-fúzió, dokumentumszűrő, darabolás, DOCX/EPUB olvasás, BibTeX-elemzés
és -párosítás, hivatkozáselemzés és oldal-átírás, REPL-elemzés, hordozható elrendezés, valamint a modellkliens
mindkét protokollra (Ollama NDJSON, OpenAI SSE) folyamaton belüli hamis szerverekkel szemben.

## Licenc

MIT © 2026 Alexio (https://trom.tf) — lásd [LICENSE](LICENSE).

## Hivatkozás-audit benchmark és hivatkozás-ellenőrzés

A `python3 variants/tests/audit_bench.py target/release/tezaur --check` a mechanikus `audit` teljesítményét méri 24 kitalált szövegrészen x 7 állításfajtán = 168 ellenőrzés (szó szerinti, parafrázis, másként írt szám, megváltoztatott szám vagy szó, tagadás, rossz forrás, összefüggéstelen). **Szintetikus és sablonokból épül**: azt mutatja, mit lát és mit nem lát a lexikai ellenőrzés, nem a valódi dolgozatokon mért pontosságot. Legutóbbi futás:

| Állításfajta | Elvárt | Helyes | Arány |
|---|---|---|---|
| verbatim | elfogadandó | 24/24 | 100 % |
| paraphrase | embernek továbbítva (nem pontozva) | 24/24 | 100 % |
| figure-form | elfogadandó | 22/24 | 92 % |
| figure-swap | jelzendő | 19/19 | 100 % |
| negation | jelzendő | 1/24 | 4 % |
| wrong-source | jelzendő | 24/24 | 100 % |
| unrelated | jelzendő | 24/24 | 100 % |
| word-swap | jelzendő | 2/5 | 40 % |

Összességében: a „jelezve, amikor kell” fedés 73 % (70/96); pontosság 97 % (70/72); téves riasztás helyes állításokon 2/48.

Értelmezés: a rossz forrásokat, az összefüggéstelen mondatokat és a megváltoztatott számokat elkapja; a tagadásokat és az egyetlen felcserélt szavakat **nem** (használd az `audit --semantic` kapcsolót helyi modellel, amelynek minősége valódi modellel nincs tesztelve); a parafrázisokat tervezetten embernek továbbítja.

A `tezaur audit draft.md --online [--fail]` ezenfelül azt is ellenőrzi, hogy minden idézett hivatkozás létezik-e és nincs-e visszavonva (OpenAlex/Crossref). Opcionális, a stderrre kiírja, mit küld, és csak az idézett `.bib` bejegyzések DOI-ját/címét küldi. Szimulált szolgáltatásokkal tesztelve; a valódi szolgáltatások a fejlesztői homokozóból nem voltak elérhetők.

## Hálózathasználat és külső eszközök

A pdf.js 3.11.174 (Apache-2.0) helyben van csomagolva a PWA-ban (`pwa/vendor/pdfjs`, a service worker gyorsítótárazza), a böngészőbővítményben (a `variants/build-extension.sh` átmásolja, a `build-web.py --local-pdfjs` pedig rá irányítja az oldalt) és a YunoHost-változatban; az egyfájlos `tezaur.html` és a `build-web.py`-jal épített többi burkoló a jsDelivr-ről tölti be Subresource Integrity hash-sel (a PDF-worker integritását nem ellenőrzi). A transformers.js (jsDelivr, dinamikus import, SRI nem lehetséges) és a Whisper tiny modell (Hugging Face) továbbra is az internetről töltődik le; a botok harmadik felek szolgáltatásain át továbbítják az üzeneteket. A „helyi” tehát a dokumentumaidra és a keresési indexre vonatkozik, nem minden változatra mindig. A PDF- és OCR-funkciókhoz telepített `pdftotext`, `pdftoppm` és `tesseract` kell. Az ajánlott modellek (`llama3.1:8b`, `bge-m3`) nincsenek csomagolva, és saját licencük van (a Llama 3.1-nek saját közösségi licence van felhasználási korlátozásokkal). Eredet: `docs/clean-room/PROVENANCE.md`.

## Támogatás

Trade-free: nincs követés, nincs reklám. Ha hasznos, támogathatod a szerzőt: https://buymeacoffee.com/echo.of.the.strings
