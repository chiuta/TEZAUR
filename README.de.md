# TEZAUR

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · **Deutsch** · [Magyar](README.hu.md)

Version: 0.5.2 · Letzte Aktualisierung: 2026-11-14

Ein lokaler, zitatgestützter Recherche-Assistent zum Schreiben einer Abschlussarbeit. Eine einzige Rust-Binärdatei.
Deine Bücher und Artikel bleiben auf deinem Rechner: Nichts wird irgendwohin gesendet, es sei denn, du führst einen der optionalen Online-Befehle aus (siehe „Netzwerknutzung“).
```
PDF / EPUB / DOCX / txt ──► chunk (page-tagged) ──► index (BM25 + optional embeddings)
   question or section title ──► retrieve ──► local model writes ONLY from the excerpts, with [n] citations
                                             └─► post-check flags invented citations, builds the reference list
```
## Warum Retrieval statt „Training auf den Büchern“

Ein Laptop kann ein Modell nicht auf einer Bibliothek feinabstimmen (Fine-Tuning), und ein feinabgestimmtes Modell würde
trotzdem Quellen erfinden. Retrieval bietet die umgekehrte Garantie: Jede Aussage verweist auf eine Passage (Datei + Seite), die du
öffnen und prüfen kannst, und das Modell wird gezwungen zu sagen, was die Bibliothek *nicht* enthält.

## Installation aus den Paketquellen (von der CI bei jedem Push gebaut und veröffentlicht)

Bei jedem Push werden diese Paketquellen auf GitHub Pages neu gebaut und vor der Veröffentlichung in sauberen Containern daraus installiert (siehe `.github/workflows/pages.yml`).

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
Signierung: Die Paketquellen werden erst GPG-signiert, wenn der Maintainer das Secret `GPG_PRIVATE_KEY` hinzufügt (siehe `deploy/signing/README.md`); bis dahin sind sie unsigniert. Der Mechanismus wird in der CI mit einem Wegwerfschlüssel getestet. Release-Archive und `.deb`-Dateien tragen schlüssellose Sigstore-Attestierungen zur Build-Provenienz: `gh attestation verify FILE -R chiuta/TEZAUR`. Der F-Droid-Index wird mit einem pro Build erzeugten Wegwerfschlüssel signiert, daher wird Android dich zur Neuinstallation auffordern, wenn er sich ändert.

## Video-Tutorials

13 kurze Tutorials (Web-App, CLI, PDF-Tabellen/-Gleichungen, Audit, MCP, Server-API, Mesh, Installation) mit Untertiteln auf Englisch, Rumänisch, Französisch, Italienisch, Spanisch, Portugiesisch, Deutsch und Ungarisch werden von `scripts/tutorials/make.py` erzeugt (siehe `scripts/tutorials/README.md`). Die Videos selbst liegen in `assets/tutorials/` (eine `.mp4` mit 8 wählbaren Untertitelspuren + je 8 `.vtt`) und laufen auf der Präsentationsseite tezaur.chiuta.ro (die installierbare Web-App liegt unter `/app/`). Die Terminalausgabe ist echt; das Modell in den Demos ist eine Attrappe, und die Übersetzungen sind ungeprüfte maschinelle Übersetzungen.

## Modell-Backends

TEZAUR braucht für Antworten einen lokalen Modellserver (nicht für `search`, `sources`, `ingest`).

| `--api`  | Server | Standardadresse |
|----------|--------|-----------------|
| `ollama` | Ollama | `http://localhost:11434` |
| `jan`    | [Jan](https://jan.ai) — Settings → Local API Server → Start Server | `http://127.0.0.1:1337/v1` |
| `openai` | LM Studio, llama.cpp `llama-server`, vLLM… (gib `--host` an) | `http://127.0.0.1:8080/v1` |

```bash
tezaur --api jan models            # lists the model ids Jan serves; use one as --model
tezaur --api jan --model <id> ask "…"
```
Oder setze `api = jan` und `model = <id>` in `Data/settings/tezaur.conf` (portabel), dann sind keine Optionen nötig.
Bei Jan/LM Studio ist das Kontextfenster eine Einstellung des Modells dort, nichts, was TEZAUR anfordern könnte.
Embeddings (`--embed-model`) funktionieren nur, wenn der Server einen Embeddings-Endpunkt anbietet; andernfalls behält ingest
deine Dokumente und die Stichwortsuche und sagt das auch.

## Einrichtung (Beispiel Ollama)

```bash
cargo build --release                      # binary: target/release/tezaur
ollama pull llama3.1:8b                    # generation; bigger model = better and slower
ollama pull bge-m3                         # optional: multilingual embeddings (Romanian/English/French)
tezaur ingest ~/thesis/library --embed-model bge-m3
```
Eingaben: **pdf, docx, epub, txt, md, html, tex**. Ein erneuter Lauf von `ingest` verarbeitet nur neue oder geänderte Dateien;
führe nach einem Upgrade von TEZAUR einmal `ingest --force` aus, um alles mit der neueren Zerlegung in Abschnitte und den neueren Lesern neu einzulesen.
Passagen werden nach Möglichkeit an Satzgrenzen geschnitten; EPUBs werden in der eigenen Lesereihenfolge des Buches gelesen.

## Befehle

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
- `--only smith` (interaktiv: `/only smith`) beschränkt alles auf Dokumente, deren Titel, Pfad oder
  Zitat passt — jeweils die Quellen eines Kapitels, wie in einem Arbeitsbereich.
- **BibTeX**: Lege einen `.bib`-Export aus Zotero / Better BibTeX neben den Index (portabel: irgendwo in `Data/`).
  Eine Datei wird über den Zitierschlüssel = Dateiname oder über das Feld `file` des Eintrags zugeordnet. Zitate lauten dann
  „Smith & Doe (2019), S. 3“, und das Literaturverzeichnis enthält die vollständigen Angaben. Eine Neuindizierung ist nicht nötig.
- **Warnung bei schwacher Übereinstimmung**: Im reinen Stichwortmodus warnt TEZAUR, wenn die beste Passage weniger als 40 % der
  Suchbegriffe abdeckt, statt das Modell ins Leere improvisieren zu lassen.

Einstellungen (Optionen / Umgebungsvariablen / `tezaur.conf`): `--api/TEZAUR_API`, `--host/TEZAUR_HOST` (Ollama auch `OLLAMA_HOST`),
`--api-key/TEZAUR_API_KEY`, `--model/TEZAUR_MODEL`, `--embed-model/TEZAUR_EMBED_MODEL`,
`--num-ctx/TEZAUR_NUM_CTX` (Ollama), `--index/TEZAUR_INDEX`, `--only`, `--bib`.

Literatureingaben und Abfragen (alles offline, sofern nicht anders angegeben):

- **Formate**: Neben `.bib` werden auch Dateien mit der Endung `.ris` und `.csl.json` im Literaturordner geladen
  (`--bib file.json` lädt ausdrücklich jede CSL-JSON-Datei). Referenztypen werden über CSL JSON abgebildet;
  BibTeX-`@string`, Verkettung mit `#` und `crossref`-Vererbung werden gelesen. DOIs, ISBNs, ISSNs und ORCID iDs
  werden extrahiert und ihre Prüfziffern verifiziert; `--unify-duplicates` lässt Einträge mit gleicher DOI/ISBN
  auf die Zitierschlüssel der jeweils anderen antworten.
- **Phrasen und Nähe** (optional): Mit `--phrases` (oder `TEZAUR_PHRASES=1`) findet `"exact phrase"` in `search`, `ask` und
  der API die Wörter in der Reihenfolge, und `"a b"~5` innerhalb von 5 Wörtern Abstand; Passagen ohne die Phrase
  werden verworfen, und ein fehlerhaftes `~k` beendet die Abfrage mit einem Fehler. Ohne die Option sind Anführungszeichen gewöhnliche
  Satzzeichen, genau wie zuvor, sodass natürlichsprachliche Fragen mit Anführungszeichen ihre Teiltreffer behalten.
  Der Abgleich ignoriert Akzente, Groß-/Kleinschreibung und Unicode-Form (der BM25-Cache wird nach dem Upgrade einmal neu aufgebaut).
- **Fundstellen** in `audit`: `[@key, p. 3]`, `pp. 3-5`, `p. 12 ff.`, römische Zahlen, Bezeichnungen `ch.`/`sec.`; ein Bereich
  wird gegen jede genannte Seite geprüft (`page-out-of-range`, `bad-range`).
- **EPUB**: Gedruckte Seitenzahlen (page-list / `epub:type="pagebreak"`) werden als Seitenzahlen verwendet, wenn das Buch sie hat.
- **Belege (Receipts)**: `ask --receipt` schreibt `tezaur-receipt/1`, wie bisher. Setze `TEZAUR_RECEIPT_FORMAT=2`, um stattdessen
  `tezaur-receipt/2` zu schreiben (CLI, HTTP-API und MCP gleichermaßen): Der Digest und der optionale HMAC (`TEZAUR_RECEIPT_KEY`)
  werden dann über die kanonische Form nach RFC 8785 gebildet, sodass eine Umformatierung der Datei keine Rolle spielt. Dokumente werden über SHA-256 benannt
  (bei der Aufnahme in einer neuen Datei `<index>.sha256` neben dem Index festgehalten; die Indexdatei selbst ist byteidentisch
  zu früher, und die Begleitdatei wird nur geschrieben, wenn ein Index geschrieben wird, nie von rein lesenden Befehlen). Dokumente, die bereits in
  einem ohne sie gebauten Index liegen, gelten als „schwache Identität“, bis sie mit `--force` erneut aufgenommen werden; ein einfaches `ingest` unveränderter
  Dateien fügt sie nicht hinzu. `PUT`/`GET /api/index` kopieren nur die Indexdatei, eine synchronisierte Kopie hat also keine SHA-256-Identitäten.
  Beide Beleg-Versionen lassen sich prüfen, unabhängig davon, welches Format gerade gewählt ist.
- **Höflichkeit gegenüber Crossref** (`refs --online`, `lit`): `--mailto you@example.org` (oder `TEZAUR_MAILTO`) trägt deine Adresse
  in den User-Agent für den „Polite Pool“ von Crossref ein; Anfragen folgen den Rate-Headern des Servers, drosseln bei 429, brechen bei 403 ab,
  und Antworten werden auf der Festplatte zwischengespeichert (30 Tage für Treffer, 1 Tag für Fehlschläge); `--refresh-online` umgeht den Cache.

## Quellenkette (FFF-Umschläge, offline)

TEZAUR ist der zweite Schritt einer Kette, die ein gescanntes Dokument ohne Netzwerk zu einer zitierfähigen Referenz führt:
`legatorocr scan.png --ocr --author "Name, Given" --fff-year 1999 --fff-out doc.fff.json` -> `tezaur ingest --fff-in doc.fff.json` -> `tezaur bib --fff-out bib.fff.json` -> `veriga --fff-in bib.fff.json --fff-out checked.fff.json` -> AcademiSearch (Library -> Receive).

- `ingest --fff-in FILE` (wiederholbar) akzeptiert einen Umschlag vom Typ `document/ocr` (Text pro Seite, Autor, Jahr, SHA-256 des Scans), `text/plain` oder `text/markdown`. Der Umschlag wird nach `fff-inbox/` neben dem Index kopiert, ein BibTeX-Eintrag `fff-<key>.bib` wird neben dem Index geschrieben (Autor, Jahr, Titel, `note = sha256:…`), und die Seiten werden indiziert, sodass Zitate `[@key, p. N]` funktionieren und `tezaur audit` sie prüfen kann. Einfache `.fff.json`-Dateien in einem an `ingest` übergebenen Ordner werden ebenfalls indiziert.
- `bib --fff-out FILE` (oder `--to fff`) schreibt die Literaturliste als Umschlag `citation/csl`, für VERIGA oder AcademiSearch.
- Grenzen: OCR-Fehler im Text werden so indiziert, wie sie sind; Autor/Jahr des Eintrags sind die, die dem vorigen Schritt gegeben wurden, ungeprüft; das Umschlagformat steht in `docs/FFF-ENVELOPE.md` in github.com/chiuta/chiuta.github.io.

## Portabel (PortableApps.com-Format)

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
Das Ausführen von `tezaur.exe` ohne Befehl öffnet den interaktiven Modus. Dokumente in `Data/` werden mit
relativem Pfad gespeichert, sodass der Ordner zwischen Laufwerken und Rechnern verschoben werden kann. Nichts wird in die Registry,
nach `%APPDATA%` oder in das Benutzerprofil geschrieben. Wenn du Jan statt Ollama verwendest, behält Jan seinen eigenen Datenordner (er lässt sich
in den Einstellungen von Jan verlegen); nur die TEZAUR-Seite ist portabel.

## HTML-Version in einer einzigen Datei

`tezaur.html` ist dieselbe Idee in einer Offline-Datei: Öffne sie in einem Browser, füge deine Dateien hinzu, richte sie auf Ollama / Jan /
LM Studio. BM25-Suche (keine Embeddings), ask / compare / gaps / summarize / outline / draft / flashcards, BibTeX,
Zitatprüfung, Anki-Export. Der Index liegt im Browser (IndexedDB). PDFs benötigen pdf.js, das einmal von einem CDN geladen wird.
Der Modellserver muss Browser-Zugriff erlauben (CORS): Ollama `OLLAMA_ORIGINS=*`, Jan → Local API Server → CORS.

### Weitere Möglichkeiten, es auszuführen

Siehe `variants/README.md`: MCP-Server, Browser-Erweiterung (Chrome/Firefox), Desktop-Apps mit Electron und Tauri, `.deb`, Docker,
YunoHost, Plugins für DokuWiki / WordPress / MediaWiki / Vim / Neovim / Emacs / Sublime / Notepad++ / Obsidian / VS Code / Joplin / Logseq /
Zotero / LibreOffice / Google Docs / Calibre, Bots für Slack / Discord / WhatsApp / Telegram / Matrix / E-Mail, PyPI- und npm-Clients, Rezepte für systemd / Helm /
Raspberry Pi / Termux / NixOS und **Mesh** (Peer-to-Peer-Austausch zwischen Forschenden, `variants/MESH.md`) — jeweils mit einem ehrlichen Status.

### Android / Smartphone

- **`pwa/`**: dieselbe Seite als installierbare Web-App (Manifest + Service Worker). Lege den Ordner auf einen beliebigen HTTPS-Host
  (z. B. dein YunoHost) und nutze Chrome → „Install app“. Funktioniert nach dem ersten Besuch offline (Seite und PDF-Lesen; der Modellserver ist getrennt, und die Browser-Diktierfunktion lädt ihr Modell bei Nutzung aus dem Internet).
- **`android/`**: ein kleines WebView-Projekt, das `tezaur.html` in eine APK packt. Es wird keine lokale Toolchain benötigt:
  GitHub → Actions → *android-apk* → Run workflow → lade `Tezaur-apk` herunter (debug-signiert, installierbar). Oder öffne
  `android/` in Android Studio. **Noch nicht gebaut oder ausgeführt** (das SDK lässt sich in meiner Sandbox nicht herunterladen).
  Der Modellserver muss vom Smartphone aus erreichbar sein (Ollama / Jan auf deinem PC, dasselbe WLAN) und CORS aktiviert haben.

## Prüfungen, die kein anderes Werkzeug dir bietet (und ihre Grenzen)

- **`audit`** liest deinen Entwurf, findet jedes `[@key, p. 3]` im Pandoc-Stil, schlägt *diese Seite* von *dieser Quelle* nach und meldet Sätze, die die
  Seite nicht abdeckt (`unsupported`, `weak`), eine wahrscheinlichere Seite (`wrong-page`), Schlüssel, die zu keinem Dokument passen, Seiten jenseits der Dokumentlänge,
  lange Aussagen ohne Zitat und Anführungszeichen, deren Text nicht wortwörtlich in der Quelle steht. Es ist ein Test auf Begriffsüberlappung: Eine Paraphrase mit
  anderen Wörtern erhält einen niedrigen Wert, und ein Satz, der die richtigen Wörter mit falscher Bedeutung wiederholt, einen hohen. Es sagt dir, wo du das Buch aufschlagen sollst.
- **`audit --semantic`** (v0.3) fragt dein lokales Modell für jeden zitierten Satz, ob die Passage dasselbe sagt (SUPPORTS / PARTIAL / CONTRADICTS / UNRELATED).
  Es kann eine korrekte Paraphrase retten und eine Verneinung erkennen, die die Wortüberlappung durchlässt; jedes von ihm geänderte Urteil ist in der Notiz mit `[model]` markiert.
  Es ist nur so gut wie das Modell, das du ausführst, und wurde hier nur gegen einen Skript-Ersatz getestet (siehe REVIEW.md).
- **`refs`** ist Buchhaltung: zitierte Schlüssel, die weder in der `.bib` noch in der Bibliothek existieren, `.bib`-Einträge und indizierte Dokumente, die dein Text nie zitiert.
  `refs --online`, `lit` und `oai` (OAI-PMH-Harvest der von dir angegebenen Repository-URL) sind die **einzigen** Befehle, die das Internet kontaktieren (OpenAlex, Crossref; `TEZAUR_MAILTO` für deren Polite Pool;
  `TEZAUR_OPENALEX` / `TEZAUR_CROSSREF`, um auf einen Mirror zu zeigen). Sie laufen nie implizit und geben eine Meldung aus, bevor sie etwas senden.
- **Tabellen und Gleichungen in PDFs** (v0.4): `ingest` hält eine Tabelle als eine Markdown-Passage zusammen (zuerst die Beschriftung, Kopfzeile wiederholt, falls geteilt werden muss) und schreibt
  abgesetzte Gleichungen als `[equation] $$ LaTeX $$ (eq. N)`; `tezaur tables FILE [--csv|--json|-o DIR]` und `tezaur equations FILE` exportieren sie. Zweispaltige Seiten werden spaltenweise gelesen.
  Es ist eine **Heuristik über Poppler-Textebene** (benötigt `pdftotext`; ohne es fällt ingest auf Klartext zurück): Spalten werden aus Lücken von 2+ Leerzeichen rekonstruiert, daher werden sehr enge Tabellen,
  verbundene/mehrzeilige Zellen, Tabellen als Bilder, gestapelte Brüche, Matrizen und tiefer-/hochgestellte Zeichen (flachgedrückt: `pi ti`) nicht rekonstruiert. Es ersetzt kein OCR-Modell wie Nougat oder MinerU.
- **`eval`** macht aus „die Suche scheint in Ordnung“ Zahlen zu deinen eigenen Fragen.
- Der Suchindex hält neben der Indexdatei einen BM25-Cache (`<index>.bm25`, automatisch neu aufgebaut, wenn sich die Bibliothek ändert): Eine CLI-Suche über 6 Mio. Wörter sank von ~4,5 s auf ~0,2 s (neu gemessen am 2026-10-30 an 6 Mio. synthetischen Wörtern, in dieser Sandbox; die erste Suche nach einem ingest zahlt die ~4,5 s für den Cache-Aufbau).
- **`coverage`** und **`novelty`** vergleichen eine Gliederung oder einen neuen Text mit dem, was deine Bibliothek tatsächlich enthält.
- **Belege** (`ask --receipt`): SHA-256 über Frage, Modell, Antwort und jeden Auszug; `receipt` prüft sie später und dass diese Auszüge im Index noch unverändert existieren.
  Ist `TEZAUR_RECEIPT_KEY` gesetzt, tragen sie zusätzlich einen HMAC. Das ist Manipulationsnachweis, keine Identitätssignatur und kein Beweis, dass eine Antwort wahr ist.
  `declare` macht aus Belegen eine einfache KI-Nutzungserklärung (Rumänisch oder Englisch) mit einer Zeile, die du selbst bestätigen musst.
- **Notebooks**: `%load_ext tezaur_client.magics` in Jupyter liefert `%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage`; `python -m tezaur_client.notebook thesis.ipynb` prüft die Prosa eines Notebooks.

Siehe [COMPETITION.md](COMPETITION.md) für den Vergleich mit heutigen Werkzeugen und [REVIEW.md](REVIEW.md) für eine gemessene, unbeschönigte Bewertung dessen, was belegt ist und was nicht (Genauigkeitszahlen des Audits, Skalierung, Sicherheitsbefunde).

## Ehrliche Grenzen

- **Kein Thesen-Schreiber.** Es entwirft aus deinen Quellen; Argumentation, Aufbau und Verifikation bleiben deine Sache.
- Die Zitatprüfung ist mechanisch: Sie erkennt nicht existierende Auszugsnummern und unzitierte lange Absätze,
  kann aber nicht sagen, ob eine zitierte Passage einen Satz wirklich stützt. **Öffne die Seite.**
- `summarize` führt Seitenverweise durch mehrere Modellaufrufe mit; TEZAUR prüft, dass sie im Dokument existieren,
  nicht dass jeder am richtigen Satz hängt. Prüfe stichprobenartig.
- `gaps` sieht nur die gefundenen Auszüge: Es meldet, was diese Passagen offenlassen, nicht alles, was fehlt.
- Kleine Modelle paraphrasieren frei. Größere Modelle (und langsamere Läufe) sind deutlich treuer.
- Gescannte PDFs brauchen zuerst OCR (z. B. `ocrmypdf`); sie werden als „keine Textebene“ gemeldet.
- EPUB/DOCX haben keine Seitenzahlen, ihre Zitate tragen daher keine Seite. EPUB-Kapitel werden in der eigenen Spine-Reihenfolge des Buches gelesen.
- Abschnitte sind Fenster von ~200 Wörtern (40 Überlappung), an Satzenden geschnitten, nicht kapitelbewusst. Tabellen und Gleichungen in PDFs werden nur heuristisch wiedergewonnen (siehe oben).
- Die Retrieval-Qualität ist die Obergrenze: Wenn `search` die richtige Passage nicht liefert, kann `ask` sie nicht nutzen.

- **Flashcards** behalten eine Karte nur, wenn ihre Antwort einen existierenden Auszug zitiert; die übrigen werden verworfen und gezählt.
  Die `.tsv` hat die Kopfzeilen `#separator:tab` / `#html:false` und zwei Spalten (Vorderseite, Rückseite + Quelle); den Import in Anki selbst konnte ich
  nicht testen.
- **Entwurfsarten** ändern den Schreibauftrag, nicht die Regeln: Jede Art zitiert weiterhin Auszüge, und `introduction`
  lässt deine eigenen Ziele als `[AUTHOR: …]`-Platzhalter stehen, statt sie zu erfinden.

## Woher die Ideen kamen

Untersucht wurden: AnythingLLM, GPT4All LocalDocs, Kotaemon, Khoj, NotebookLM, PrivateGPT, Zotero-basierte Assistenten und die
(inzwischen aufgegebene) Linux-Distribution UberStudent für Studierende. Von UberStudent: ein nach akademischen Fertigkeiten gegliedertes Menü
(lesen / recherchieren / schreiben / lernen — das Layout von `/help`), Lernhilfen rund um Anki, Vorlagen für akademische Arbeiten
(`draft --kind`) und eine Anleitung für den ersten Start (`doctor`, der Hinweis „nächster Schritt“). UberStudent lief außerdem live von einem USB-Stick,
wofür das portable Layout gedacht ist.

## CI

- `.github/workflows/ci.yml` — bei jedem Push/PR: Build, `cargo test`, Bau aller Plugins und Lauf der gesamten Integrationssuite (`variants/tests/run-all.sh`, mit installierten poppler und pdflatex, damit die PDF-Tests laufen); baut und testet außerdem unter Windows und macOS.
- `.github/workflows/release.yml` — von Hand oder bei einem Tag `v*`: CLI (3 Betriebssysteme), .deb, .rpm, Electron, Tauri, Windows-Installer, Erweiterungen, Obsidian/VS Code/Office, Capacitor-APK und jedes Plugin/jede Integration (`variants/build-plugins.sh`); bei einem Tag hängt ein letzter Job alle Artefakte und eine `SHA256SUMS.txt` an das GitHub-Release.
- `portable.yml`, `android.yml`, `docker.yml`, `pages.yml` — Windows-PortableApps-Ordner, Android, Docker-Image, PWA auf GitHub Pages.

**Keiner dieser Workflows ist bisher auf GitHub gelaufen** (nur ihre Schritte wurden lokal unter Linux durchgespielt); rechne damit, beim ersten Lauf einen Pfad oder ein fehlendes Paket korrigieren zu müssen, besonders bei Windows, macOS, Electron, Tauri, iOS und Android.

## Bewusst nicht gebaut

- **Modellantworten als Quellen in die Bibliothek zurückspeichern**: Die nächste Antwort könnte dann den eigenen früheren
  Text des Modells wie einen Beleg zitieren. Speichere Entwürfe mit `-o` und nimm sie nur auf, wenn du das in Kauf nimmst.
- **Cloud-Dienste für Embeddings / Reranking**: widersprechen dem Sinn eines lokalen Werkzeugs. Ein lokaler Reranker ist eine mögliche
  spätere Ergänzung.

## Tests

`cargo test` (61 Tests, darunter SHA-256/HMAC-Vektoren und die Audit-/Beleg-Logik) und `variants/tests/run-all.sh` — Tokenizer/Diakritika, BM25, Rank Fusion, Dokumentfilter, Zerlegung in Abschnitte, DOCX-/EPUB-Lesen, BibTeX-Parsing
und -Abgleich, Zitat-Parsing und Seitenumschreibung, REPL-Parsing, portables Layout und der Modell-Client
für beide Protokolle (Ollama NDJSON, OpenAI SSE) gegen prozessinterne Fake-Server.

## Lizenz

MIT © 2026 Alexio (https://trom.tf) — siehe [LICENSE](LICENSE).

## Benchmark der Zitatprüfung und Referenzprüfung

`python3 variants/tests/audit_bench.py target/release/tezaur --check` misst den mechanischen `audit` an 24 erfundenen Passagen x 7 Arten von Aussagen = 168 Prüfungen (wörtlich, Paraphrase, anders geschriebene Zahl, geänderte Zahl oder geändertes Wort, Verneinung, falsche Quelle, ohne Bezug). Er ist **synthetisch und aus Vorlagen gebaut**: Er zeigt, was die lexikalische Prüfung sehen kann und was nicht, nicht die Genauigkeit an echten Abschlussarbeiten. Letzter Lauf:

| Art der Aussage | Erwartet | Richtig | Rate |
|---|---|---|---|
| verbatim | sollte akzeptiert werden | 24/24 | 100 % |
| paraphrase | an einen Menschen weitergegeben (nicht bewertet) | 24/24 | 100 % |
| figure-form | sollte akzeptiert werden | 22/24 | 92 % |
| figure-swap | sollte markiert werden | 19/19 | 100 % |
| negation | sollte markiert werden | 1/24 | 4 % |
| wrong-source | sollte markiert werden | 24/24 | 100 % |
| unrelated | sollte markiert werden | 24/24 | 100 % |
| word-swap | sollte markiert werden | 2/5 | 40 % |

Insgesamt: Recall „markiert, wenn es markiert werden soll“ 73 % (70/96); Precision 97 % (70/72); Fehlalarme bei korrekten Aussagen 2/48.

Zum Lesen: Falsche Quellen, unzusammenhängende Sätze und geänderte Zahlen werden erkannt; Verneinungen und einzelne vertauschte Wörter **nicht** (nutze `audit --semantic` mit einem lokalen Modell, dessen Qualität mit einem echten Modell nicht getestet ist); Paraphrasen werden absichtlich an einen Menschen weitergegeben.

`tezaur audit draft.md --online [--fail]` prüft zusätzlich, dass jede zitierte Referenz existiert und nicht zurückgezogen wurde (OpenAlex/Crossref). Es ist optional, gibt auf stderr aus, was es sendet, und sendet nur DOI/Titel der zitierten `.bib`-Einträge. Gegen simulierte Dienste getestet; die echten Dienste waren aus der Entwicklungs-Sandbox nicht erreichbar.

## Netzwerknutzung und externe Werkzeuge

pdf.js 3.11.174 (Apache-2.0) ist lokal in der PWA gebündelt (`pwa/vendor/pdfjs`, vom Service Worker zwischengespeichert), in der Browser-Erweiterung (`variants/build-extension.sh` kopiert es, und `build-web.py --local-pdfjs` richtet die Seite darauf aus) und in der YunoHost-Variante; die Ein-Datei-`tezaur.html` und die anderen mit `build-web.py` gebauten Wrapper laden es von jsDelivr mit einem Subresource-Integrity-Hash (der PDF-Worker ist nicht integritätsgeprüft). transformers.js (jsDelivr, dynamischer Import, kein SRI möglich) und das Whisper-Tiny-Modell (Hugging Face) werden weiterhin aus dem Internet geladen; Bots leiten Nachrichten über Dienste Dritter weiter. „Lokal“ gilt daher für deine Dokumente und den Retrieval-Index, nicht für jede Variante zu jeder Zeit. PDF- und OCR-Funktionen benötigen installierte `pdftotext`, `pdftoppm` und `tesseract`. Empfohlene Modelle (`llama3.1:8b`, `bge-m3`) sind nicht enthalten und haben eigene Lizenzen (Llama 3.1 hat eine eigene Community-Lizenz mit Nutzungsbeschränkungen). Provenienz: `docs/clean-room/PROVENANCE.md`.

## Unterstützung

Trade-free: kein Tracking, keine Werbung. Wenn es dir nützt, kannst du den Autor unterstützen: https://buymeacoffee.com/echo.of.the.strings
