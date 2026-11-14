# TEZAUR

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · **Español** · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versión: 0.5.2 · Última actualización: 2026-11-14

Un asistente de investigación local, basado en citas, para escribir una tesis. Un solo binario de Rust.
Tus libros y artículos se quedan en tu equipo: no se envía nada a ninguna parte salvo que ejecutes uno de los comandos en línea opcionales (consulta «Uso de la red»).
```
PDF / EPUB / DOCX / txt ──► chunk (page-tagged) ──► index (BM25 + optional embeddings)
   question or section title ──► retrieve ──► local model writes ONLY from the excerpts, with [n] citations
                                             └─► post-check flags invented citations, builds the reference list
```
## Por qué recuperación y no «entrenar con los libros»

Un portátil no puede hacer fine-tuning de un modelo con una biblioteca, y un modelo afinado
seguiría inventando fuentes. La recuperación ofrece la garantía opuesta: cada afirmación apunta a un pasaje (archivo + página) que puedes
abrir y comprobar, y el modelo está obligado a decir lo que la biblioteca *no* contiene.

## Instalar desde los repositorios de paquetes (compilados y publicados por la CI en cada push)

En cada push, estos repositorios se reconstruyen en GitHub Pages y se instalan desde ellos en contenedores limpios antes de publicar (consulta `.github/workflows/pages.yml`).

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
Firma: los repositorios solo se firman con GPG cuando el mantenedor añade el secreto `GPG_PRIVATE_KEY` (consulta `deploy/signing/README.md`); hasta entonces no están firmados. El mecanismo se prueba en la CI con una clave desechable. Los archivos de versión y los `.deb` llevan atestaciones de procedencia de compilación Sigstore sin claves: `gh attestation verify FILE -R chiuta/TEZAUR`. El índice de F-Droid se firma con una clave desechable generada en cada compilación, de modo que Android te pedirá reinstalar cuando cambie.

## Tutoriales en vídeo

13 tutoriales breves (aplicación web, CLI, tablas/ecuaciones de PDF, auditoría, MCP, API del servidor, Mesh, instalación) con subtítulos en inglés, rumano, francés, italiano, español, portugués, alemán y húngaro se generan con `scripts/tutorials/make.py` (consulta `scripts/tutorials/README.md`). Los vídeos están en `assets/tutorials/` (un `.mp4` con 8 pistas de subtítulos seleccionables + 8 `.vtt` cada uno) y se reproducen en la página de presentación, tezaur.chiuta.ro (la aplicación web instalable está en `/app/`). La salida del terminal es real; el modelo de las demostraciones es simulado, y las traducciones son traducciones automáticas sin revisar.

## Motores de modelos

TEZAUR necesita un servidor de modelos local para las respuestas (no para `search`, `sources`, `ingest`).

| `--api`  | Servidor | Dirección predeterminada |
|----------|----------|--------------------------|
| `ollama` | Ollama | `http://localhost:11434` |
| `jan`    | [Jan](https://jan.ai) — Settings → Local API Server → Start Server | `http://127.0.0.1:1337/v1` |
| `openai` | LM Studio, llama.cpp `llama-server`, vLLM… (indica `--host`) | `http://127.0.0.1:8080/v1` |

```bash
tezaur --api jan models            # lists the model ids Jan serves; use one as --model
tezaur --api jan --model <id> ask "…"
```
O pon `api = jan` y `model = <id>` en `Data/settings/tezaur.conf` (portátil) para no necesitar opciones.
Con Jan/LM Studio, la ventana de contexto es un ajuste del modelo allí, no algo que TEZAUR pueda solicitar.
Los embeddings (`--embed-model`) solo funcionan si el servidor ofrece un endpoint de embeddings; si no, ingest conserva
tus documentos y la búsqueda por palabras clave, y lo indica.

## Configuración (ejemplo con Ollama)

```bash
cargo build --release                      # binary: target/release/tezaur
ollama pull llama3.1:8b                    # generation; bigger model = better and slower
ollama pull bge-m3                         # optional: multilingual embeddings (Romanian/English/French)
tezaur ingest ~/thesis/library --embed-model bge-m3
```
Entradas: **pdf, docx, epub, txt, md, html, tex**. Volver a ejecutar `ingest` solo procesa los archivos nuevos o modificados;
tras actualizar TEZAUR, ejecuta `ingest --force` una vez para releerlo todo con la nueva división en fragmentos y los nuevos lectores.
Los pasajes se cortan en los límites de las frases cuando es posible; los EPUB se leen en el orden de lectura propio del libro.

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
- `--only smith` (en modo interactivo: `/only smith`) restringe todo a los documentos cuyo título, ruta o
  cita coincida — las fuentes de un solo capítulo cada vez, como un espacio de trabajo.
- **BibTeX**: deja una exportación `.bib` de Zotero / Better BibTeX junto al índice (portátil: en cualquier lugar de `Data/`).
  Un archivo se asocia por clave de cita = nombre del archivo, o por el campo `file` de la entrada. Entonces las citas se leen
  «Smith & Doe (2019), p. 3» y la lista de referencias lleva todos los detalles. No hace falta reindexar.
- **Aviso de coincidencia débil**: en el modo solo de palabras clave, TEZAUR avisa cuando el mejor pasaje cubre menos del 40 % de
  los términos de la consulta, en lugar de dejar que el modelo improvise sobre la nada.

Ajustes (opciones / variables de entorno / `tezaur.conf`): `--api/TEZAUR_API`, `--host/TEZAUR_HOST` (Ollama también `OLLAMA_HOST`),
`--api-key/TEZAUR_API_KEY`, `--model/TEZAUR_MODEL`, `--embed-model/TEZAUR_EMBED_MODEL`,
`--num-ctx/TEZAUR_NUM_CTX` (Ollama), `--index/TEZAUR_INDEX`, `--only`, `--bib`.

Entradas bibliográficas y consultas (todo sin conexión salvo que se indique lo contrario):

- **Formatos**: además de `.bib`, también se cargan los archivos que terminan en `.ris` y `.csl.json` de la carpeta de la bibliografía
  (`--bib file.json` carga explícitamente cualquier archivo CSL JSON). Los tipos de referencia se asignan mediante CSL JSON;
  se leen los `@string` de BibTeX, la concatenación con `#` y la herencia `crossref`. Se extraen los DOI, ISBN, ISSN y ORCID iD
  y se verifican sus dígitos de control; `--unify-duplicates` permite que las entradas con el mismo DOI/ISBN
  respondan a las claves de cita de las demás.
- **Frases y proximidad** (opcional): con `--phrases` (o `TEZAUR_PHRASES=1`), `"exact phrase"` en `search`, `ask` y
  la API busca las palabras en orden, y `"a b"~5` a no más de 5 palabras de distancia; los pasajes sin la frase
  se descartan, y un `~k` mal formado detiene la consulta con un error. Sin la opción, las comillas son puntuación normal,
  exactamente como antes, de modo que las preguntas en lenguaje natural que contienen comillas conservan sus coincidencias parciales.
  La coincidencia ignora acentos, mayúsculas y forma Unicode (la caché BM25 se reconstruye una vez tras la actualización).
- **Localizadores** en `audit`: `[@key, p. 3]`, `pp. 3-5`, `p. 12 ff.`, números romanos, etiquetas `ch.`/`sec.`; un intervalo
  se comprueba en cada página que nombra (`page-out-of-range`, `bad-range`).
- **EPUB**: los números de página impresos (page-list / `epub:type="pagebreak"`) se usan como números de página cuando el libro los tiene.
- **Recibos**: `ask --receipt` escribe `tezaur-receipt/1`, como antes. Define `TEZAUR_RECEIPT_FORMAT=2` para escribir
  en su lugar `tezaur-receipt/2` (CLI, API HTTP y MCP por igual): el resumen y el HMAC opcional (`TEZAUR_RECEIPT_KEY`)
  se calculan entonces sobre la forma canónica RFC 8785, así que reformatear el archivo no importa. Los documentos se nombran por SHA-256
  (registrado en la ingesta en un archivo nuevo `<index>.sha256` junto al índice; el archivo de índice en sí es idéntico byte a byte
  al de antes, y el archivo acompañante solo se escribe cuando se escribe un índice, nunca por los comandos de solo lectura). Los documentos ya presentes en
  un índice construido sin él se marcan como «identidad débil» hasta que se reingieren con `--force`; un `ingest` simple de
  archivos sin cambios no lo añade. `PUT`/`GET /api/index` copian solo el archivo de índice, por lo que una copia sincronizada no tiene identidades SHA-256.
  Ambas versiones de recibo se verifican, sea cual sea el formato seleccionado en ese momento.
- **Cortesía con Crossref** (`refs --online`, `lit`): `--mailto you@example.org` (o `TEZAUR_MAILTO`) pone tu dirección
  en el User-Agent para el «polite pool» de Crossref; las peticiones siguen las cabeceras de frecuencia del servidor, retroceden ante 429, se detienen ante 403,
  y las respuestas se guardan en caché en disco (30 días los aciertos, 1 día los fallos); `--refresh-online` omite la caché.

## Cadena de fuentes (sobres FFF, sin conexión)

TEZAUR es el segundo paso de una cadena que lleva un documento escaneado a una referencia citable sin red:
`legatorocr scan.png --ocr --author "Name, Given" --fff-year 1999 --fff-out doc.fff.json` -> `tezaur ingest --fff-in doc.fff.json` -> `tezaur bib --fff-out bib.fff.json` -> `veriga --fff-in bib.fff.json --fff-out checked.fff.json` -> AcademiSearch (Library -> Receive).

- `ingest --fff-in FILE` (repetible) acepta un sobre de tipo `document/ocr` (texto por página, autor, año, SHA-256 del escaneo), `text/plain` o `text/markdown`. El sobre se copia en `fff-inbox/` junto al índice, se escribe una entrada BibTeX `fff-<key>.bib` junto al índice (autor, año, título, `note = sha256:…`) y se indexan las páginas, de modo que las citas `[@key, p. N]` funcionan y `tezaur audit` puede comprobarlas. Los archivos `.fff.json` simples de una carpeta pasada a `ingest` también se indexan.
- `bib --fff-out FILE` (o `--to fff`) escribe la bibliografía como un sobre `citation/csl`, para VERIGA o AcademiSearch.
- Límites: los errores de OCR del texto se indexan tal cual; el autor/año de la entrada son los que se dieron al paso anterior, no se comprueban; el formato del sobre está en `docs/FFF-ENVELOPE.md` de github.com/chiuta/chiuta.github.io.

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
Ejecutar `tezaur.exe` sin comando abre el modo interactivo. Los documentos dentro de `Data/` se almacenan por
ruta relativa, de modo que la carpeta puede moverse entre unidades y equipos. No se escribe nada en el registro,
en `%APPDATA%` ni en el perfil de usuario. Si usas Jan en lugar de Ollama, Jan mantiene su propia carpeta de datos (se puede
reubicar en los ajustes de Jan); solo el lado de TEZAUR es portátil.

## Versión HTML de un solo archivo

`tezaur.html` es la misma idea en un único archivo sin conexión: ábrelo en un navegador, añade tus archivos, apúntalo a Ollama / Jan /
LM Studio. Búsqueda BM25 (sin embeddings), ask / compare / gaps / summarize / outline / draft / flashcards, BibTeX,
comprobación de citas, exportación a Anki. El índice vive en el navegador (IndexedDB). Los PDF necesitan pdf.js, que se descarga una vez desde una CDN.
El servidor de modelos debe permitir el acceso desde el navegador (CORS): Ollama `OLLAMA_ORIGINS=*`, Jan → Local API Server → CORS.

### Otras formas de ejecutarlo

Consulta `variants/README.md`: servidor MCP, extensión de navegador (Chrome/Firefox), aplicaciones de escritorio Electron y Tauri, `.deb`, Docker,
YunoHost, plugins para DokuWiki / WordPress / MediaWiki / Vim / Neovim / Emacs / Sublime / Notepad++ / Obsidian / VS Code / Joplin / Logseq /
Zotero / LibreOffice / Google Docs / Calibre, bots de Slack / Discord / WhatsApp / Telegram / Matrix / correo electrónico, clientes PyPI + npm, recetas de systemd / Helm /
Raspberry Pi / Termux / NixOS, y **Mesh** (intercambio entre pares entre investigadores, `variants/MESH.md`) — con un estado honesto para cada uno.

### Android / teléfono

- **`pwa/`**: la misma página como aplicación web instalable (manifiesto + service worker). Pon la carpeta en cualquier host HTTPS
  (por ejemplo tu YunoHost) y usa Chrome → «Install app». Funciona sin conexión tras la primera visita (página y lectura de PDF; el servidor de modelos es aparte, y el dictado del navegador descarga su modelo de internet cuando se usa).
- **`android/`**: un pequeño proyecto WebView que incluye `tezaur.html` en un APK. No hace falta ninguna cadena de herramientas local:
  GitHub → Actions → *android-apk* → Run workflow → descarga `Tezaur-apk` (firmado en modo depuración, instalable). O abre
  `android/` en Android Studio. **Aún no se ha compilado ni ejecutado** (el SDK no se puede descargar en mi entorno aislado).
  El servidor de modelos debe ser accesible desde el teléfono (Ollama / Jan en tu PC, la misma red Wi-Fi) con CORS activado.

## Comprobaciones que ninguna otra herramienta te ofrece (y sus límites)

- **`audit`** lee tu borrador, encuentra cada `[@key, p. 3]` al estilo pandoc, consulta *esa página* de *esa fuente* e informa de las frases que la
  página no cubre (`unsupported`, `weak`), de una página más probable (`wrong-page`), de claves que no coinciden con ningún documento, de páginas más allá de la longitud del documento,
  de afirmaciones largas sin cita y de comillas cuyo texto no figura palabra por palabra en la fuente. Es una prueba de solapamiento de términos: una paráfrasis que
  use otras palabras puntuará bajo, y una frase que repita las palabras correctas con el significado equivocado puntuará alto. Te dice dónde abrir el libro.
- **`audit --semantic`** (v0.3) pregunta a tu modelo local, para cada frase citada, si el pasaje dice lo mismo (SUPPORTS / PARTIAL / CONTRADICTS / UNRELATED).
  Puede rescatar una paráfrasis correcta y detectar una negación que el solapamiento de palabras acepta; cada veredicto que cambia se marca `[model]` en la nota.
  Es tan bueno como el modelo que ejecutes, y aquí solo se ha probado con un sustituto guionizado (consulta REVIEW.md).
- **`refs`** es contabilidad: claves citadas que no existen ni en el `.bib` ni en la biblioteca, entradas `.bib` y documentos indexados que tu texto nunca cita.
  `refs --online`, `lit` y `oai` (cosecha OAI-PMH de la URL del repositorio que indiques) son los **únicos** comandos que contactan con internet (OpenAlex, Crossref; `TEZAUR_MAILTO` para su «polite pool»;
  `TEZAUR_OPENALEX` / `TEZAUR_CROSSREF` para apuntar a un espejo). Nunca se ejecutan de forma implícita e imprimen un aviso antes de enviar nada.
- **Tablas y ecuaciones en PDF** (v0.4): `ingest` mantiene una tabla unida como un solo pasaje Markdown (primero el pie, con la cabecera repetida si hay que dividirla) y escribe
  las ecuaciones destacadas como `[equation] $$ LaTeX $$ (eq. N)`; `tezaur tables FILE [--csv|--json|-o DIR]` y `tezaur equations FILE` las exportan. Las páginas de dos columnas se leen columna por columna.
  Es una **heurística sobre la capa de texto de poppler** (necesita `pdftotext`; sin él, ingest recurre a texto plano): las columnas se recuperan a partir de huecos de 2 o más espacios, así que las tablas muy ajustadas,
  las celdas combinadas/de varias líneas, las tablas que son imágenes, las fracciones apiladas, las matrices y los subíndices/superíndices (aplanados: `pi ti`) no se reconstruyen. No sustituye a un modelo de OCR como Nougat o MinerU.
- **`eval`** convierte «la búsqueda parece ir bien» en cifras sobre tus propias preguntas.
- El índice de búsqueda mantiene una caché BM25 junto al archivo de índice (`<index>.bm25`, reconstruida automáticamente cuando cambia la biblioteca): una búsqueda CLI sobre 6 M de palabras pasó de ~4,5 s a ~0,2 s (vuelto a medir el 2026-10-30 sobre 6 M de palabras sintéticas, en este entorno; la primera búsqueda tras un ingest paga los ~4,5 s de construcción de la caché).
- **`coverage`** y **`novelty`** comparan un esquema o un texto nuevo con lo que tu biblioteca contiene realmente.
- **Recibos** (`ask --receipt`): SHA-256 sobre la pregunta, el modelo, la respuesta y cada extracto; `receipt` lo verifica más tarde, junto con que esos extractos sigan existiendo sin cambios
  en el índice. Con `TEZAUR_RECEIPT_KEY` definida, llevan además un HMAC. Esto es evidencia de manipulación, no una firma de identidad, ni prueba de que una respuesta sea verdadera.
  `declare` convierte los recibos en una sencilla declaración de uso de IA (en rumano o inglés) con una línea que debes confirmar tú mismo.
- **Notebooks**: `%load_ext tezaur_client.magics` en Jupyter ofrece `%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage`; `python -m tezaur_client.notebook thesis.ipynb` audita la prosa de un notebook.

Consulta [COMPETITION.md](COMPETITION.md) para ver cómo se compara con las herramientas actuales, y [REVIEW.md](REVIEW.md) para una revisión medida y sin concesiones de lo que está y no está demostrado (cifras de precisión de la auditoría, escala, hallazgos de seguridad).

## Límites honestos

- **No es un escritor de tesis.** Redacta a partir de tus fuentes; el argumento, la estructura y la verificación son tuyos.
- La comprobación de citas es mecánica: detecta números de extracto inexistentes y párrafos largos sin citar,
  pero no puede saber si un pasaje citado respalda de verdad una frase. **Abre la página.**
- `summarize` arrastra las referencias de página a través de varias llamadas al modelo; TEZAUR comprueba que existen en el documento,
  no que cada una esté unida a la frase correcta. Revisa por muestreo.
- `gaps` solo ve los extractos recuperados: informa de lo que esos pasajes dejan abierto, no de todo lo que falta.
- Los modelos pequeños parafrasean con laxitud. Los modelos más grandes (y las ejecuciones más lentas) son claramente más fieles.
- Los PDF escaneados necesitan OCR primero (por ejemplo `ocrmypdf`); se indican como «sin capa de texto».
- EPUB/DOCX no tienen números de página, así que sus citas no llevan página. Los capítulos EPUB se leen en el orden propio del libro (spine).
- Los fragmentos son ventanas de ~200 palabras (40 de solapamiento) cortadas al final de las frases, sin tener en cuenta los capítulos. Las tablas y ecuaciones de los PDF solo se recuperan de forma heurística (véase más arriba).
- La calidad de la recuperación es el techo: si `search` no hace aflorar el pasaje correcto, `ask` no puede usarlo.

- **Flashcards** conserva una tarjeta solo si su respuesta cita un extracto que existe; el resto se descartan y se cuentan.
  El `.tsv` tiene las líneas de cabecera `#separator:tab` / `#html:false` y dos columnas (anverso, reverso + fuente); no pude
  probar la importación en el propio Anki.
- **Los tipos de borrador** cambian el encargo de redacción, no las reglas: cada tipo sigue citando extractos, y `introduction`
  deja tus propios objetivos como un marcador `[AUTHOR: …]` en lugar de inventarlos.

## De dónde vienen las ideas

Examinados: AnythingLLM, GPT4All LocalDocs, Kotaemon, Khoj, NotebookLM, PrivateGPT, los asistentes basados en Zotero y la
(ahora abandonada) distribución Linux para estudiantes UberStudent. De UberStudent: un menú organizado por habilidad académica
(leer / investigar / escribir / estudiar — la disposición de `/help`), ayudas de estudio construidas en torno a Anki, plantillas para trabajos académicos
(`draft --kind`) y una guía de primer uso (`doctor`, la pista del «siguiente paso»). UberStudent también funcionaba en vivo desde una memoria USB,
que es para lo que sirve la disposición portátil.

## CI

- `.github/workflows/ci.yml` — en cada push/PR: compilación, `cargo test`, compila todos los plugins y ejecuta toda la suite de integración (`variants/tests/run-all.sh`, con poppler y pdflatex instalados para que se ejecuten las pruebas de PDF); también compila y prueba en Windows y macOS.
- `.github/workflows/release.yml` — a mano o con una etiqueta `v*`: CLI (3 sistemas operativos), .deb, .rpm, Electron, Tauri, instalador de Windows, extensiones, Obsidian/VS Code/Office, APK de Capacitor y todos los plugins/integraciones (`variants/build-plugins.sh`); con una etiqueta, un trabajo final adjunta todos los artefactos y un `SHA256SUMS.txt` a la versión de GitHub.
- `portable.yml`, `android.yml`, `docker.yml`, `pages.yml` — carpeta Windows PortableApps, Android, imagen Docker, PWA en GitHub Pages.

**Ninguno de estos flujos de trabajo se ha ejecutado aún en GitHub** (solo sus pasos se ejercitaron en local sobre Linux); espera tener que corregir una ruta o un paquete que falte en la primera ejecución, sobre todo en Windows, macOS, Electron, Tauri, iOS y Android.

## Deliberadamente no construido

- **Guardar las respuestas del modelo en la biblioteca como fuentes**: la siguiente respuesta podría entonces citar el propio
  texto anterior del modelo como si fuera una prueba. Guarda los borradores con `-o`, e ingiérelos solo si aceptas eso.
- **Servicios en la nube de embeddings / reranking**: contrarios al propósito de una herramienta local. Un reranker local es una posible
  incorporación futura.

## Pruebas

`cargo test` (61 pruebas, incluidos vectores SHA-256/HMAC y la lógica de auditoría/recibos) y `variants/tests/run-all.sh` — tokenizador/diacríticos, BM25, fusión de clasificaciones, filtro de documentos, división en fragmentos, lectura de DOCX/EPUB, análisis
y coincidencia de BibTeX, análisis de citas y reescritura de páginas, análisis del REPL, disposición portátil y cliente de modelos
para ambos protocolos (Ollama NDJSON, OpenAI SSE) frente a servidores falsos en proceso.

## Licencia

MIT © 2026 Alexio (https://trom.tf) — consulta [LICENSE](LICENSE).

## Referencia de auditoría de citas y comprobación de referencias

`python3 variants/tests/audit_bench.py target/release/tezaur --check` mide la `audit` mecánica sobre 24 pasajes inventados x 7 tipos de afirmación = 168 comprobaciones (literal, paráfrasis, cifra escrita de otra forma, cifra o palabra cambiada, negación, fuente equivocada, sin relación). Es **sintético y construido a partir de plantillas**: muestra lo que la comprobación léxica puede y no puede ver, no la precisión en tesis reales. Última ejecución:

| Tipo de afirmación | Esperado | Correcto | Tasa |
|---|---|---|---|
| verbatim | debe aceptarse | 24/24 | 100 % |
| paraphrase | enviado a una persona (sin puntuar) | 24/24 | 100 % |
| figure-form | debe aceptarse | 22/24 | 92 % |
| figure-swap | debe señalarse | 19/19 | 100 % |
| negation | debe señalarse | 1/24 | 4 % |
| wrong-source | debe señalarse | 24/24 | 100 % |
| unrelated | debe señalarse | 24/24 | 100 % |
| word-swap | debe señalarse | 2/5 | 40 % |

En conjunto: exhaustividad de «señalado cuando debe serlo» 73 % (70/96); precisión 97 % (70/72); falsas alarmas sobre afirmaciones correctas 2/48.

Cómo leerlo: las fuentes equivocadas, las frases sin relación y las cifras cambiadas se detectan; las negaciones y las palabras sueltas intercambiadas **no** (usa `audit --semantic` con un modelo local, cuya calidad no se ha probado con un modelo real); las paráfrasis se envían a una persona por diseño.

`tezaur audit draft.md --online [--fail]` comprueba además que cada referencia citada exista y no esté retractada (OpenAlex/Crossref). Es opcional, imprime en stderr lo que envía y envía solo el DOI/título de las entradas `.bib` citadas. Probado con servicios simulados; los servicios reales no eran accesibles desde el entorno aislado de desarrollo.

## Uso de la red y herramientas externas

pdf.js 3.11.174 (Apache-2.0) se incluye localmente en la PWA (`pwa/vendor/pdfjs`, guardado en caché por su service worker), en la extensión de navegador (`variants/build-extension.sh` lo copia y `build-web.py --local-pdfjs` apunta la página hacia él) y en la variante de YunoHost; el `tezaur.html` de un solo archivo y los demás envoltorios construidos con `build-web.py` lo cargan desde jsDelivr con un hash de Subresource Integrity (el worker de PDF no tiene comprobación de integridad). transformers.js (jsDelivr, importación dinámica, sin SRI posible) y el modelo Whisper tiny (Hugging Face) todavía se descargan de internet; los bots reenvían los mensajes a través de servicios de terceros. Por tanto, «local» se aplica a tus documentos y al índice de recuperación, no a todas las variantes en todo momento. Las funciones de PDF y OCR necesitan `pdftotext`, `pdftoppm` y `tesseract` instalados. Los modelos recomendados (`llama3.1:8b`, `bge-m3`) no se incluyen y tienen sus propias licencias (Llama 3.1 tiene su propia licencia comunitaria con restricciones de uso). Procedencia: `docs/clean-room/PROVENANCE.md`.

## Apoyo

Trade-free: sin rastreo, sin anuncios. Si te resulta útil, puedes apoyar al autor: https://buymeacoffee.com/echo.of.the.strings
