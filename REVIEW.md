# TEZAUR — recenzie onestă a ceea ce a fost implementat

Scrisă de cel care a scris codul, deci cu riscul unei indulgențe pe care am încercat s-o compensez măsurând, nu opinând.
Starea la versiunea 0.3.0 (secțiunea „Runda 0.3.0” de la sfârșit descrie ce s-a schimbat; restul textului e scris la 0.2.x și rămâne valabil unde nu e corectat). „Testat" înseamnă că am rulat efectiv; „neverificat" înseamnă că nu am putut sau nu am rulat.

## Verdict în patru rânduri

1. **Nucleul e bun și simplu**: ingestie → indexare BM25 (+ embeddings opționale) → pasaje cu pagină → model local care are voie să scrie doar din ele →
   verificare mecanică a citărilor. Ideea corectă pentru o teză: nu „antrenezi pe cărți", ci ții fiecare afirmație legată de o pagină.
2. **Nu am rulat niciodată proiectul cu un model de limbaj real.** Toate testele de „ask / compare / gaps / draft" folosesc un model simulat care
   răspunde mereu cu text citat. Calitatea reală a răspunsurilor, respectarea instrucțiunilor de către modele mici și utilitatea `--rerank`/`--verify` sunt **necunoscute**.
3. **Suprafața s-a extins mult mai repede decât validarea.** Aproximativ 40 de integrări (CMS-uri, editoare, boți, magazine de aplicații) există ca
   fișiere scrise cu grijă, dar aproape toate au fost verificate doar contra unor simulări. Fiecare este o datorie de întreținere, nu o funcție dovedită.
4. **Verificările noi (audit, coverage, novelty, chitanțe) sunt utile dar limitate, iar limitele sunt măsurate mai jos** — nu sunt dovezi, sunt semnalizatoare.

## Ce este solid (și de ce cred asta)

| Zonă | Dovadă |
|---|---|
| Indexare/căutare/citare în Rust | 58 de teste unitare; rulări cap-coadă prin CLI, API, MCP; diacritice românești; PDF/EPUB/DOCX reale de test |
| Verificări mecanice ale citărilor | benchmark reproductibil (`variants/tests/audit_benchmark.py`), rezultate mai jos |
| Chitanțe | SHA-256/HMAC verificate cu vectorii standard; modificarea oricărui câmp e detectată (test) |
| Mesh P2P | două servere reale, token refuzat, documentele nepartajate nu ies, dedup, pachete `.tzb`, descoperire UDP, SOCKS, WebRTC între două contexte Chromium |
| Export | PDF compilat cu pdflatex+bibtex cu citările rezolvate; ODT/EPUB prin pandoc; RIS/CSL/BibTeX; .apkg citit înapoi |
| Installer/uninstaller | pe un „home" fals cu aplicații false: instalare, a doua rulare idempotentă, dezinstalare care readuce configurările, `--sweep` fără registru, refuz să șteargă date fără `--yes` |
| Notebook Jupyter | executat într-un kernel real cu un server real |

## Măsurători (nu impresii)

### 1. Cât de bun este `audit`? (25 de afirmații pe 8 pasaje scrise de mine)
| Tip de afirmație | Rezultat |
|---|---|
| Citată aproape verbatim din sursa corectă | **6/6** acceptate |
| Fără legătură cu sursa (pinguini, Rust, sonate) | **4/4** semnalate |
| Citată la o sursă greșită | **3/3** semnalate |
| **Parafrazată cu alte cuvinte, dar corectă** | **0/6 acceptate** — toate semnalate „weak/unsupported" (alarmă falsă) |
| Contrazice sursa schimbând o cifră („10 ani" în loc de 3) | **3/3** prinse (după ce am adăugat controlul cifrelor în această rundă; înainte: 0/3) |
| Contrazice sursa prin negație/polaritate („nu înlocuiește" ↔ „înlocuiește") | **0/2 prinse** — scor de suprapunere 0.92–1.0, acceptate ca „supported" |
| Contrazice sursa folosind alte cuvinte (1 caz) | semnalată „weak", dar doar pentru că suprapunerea lexicală era mică — din motive greșite |

Ce înseamnă: auditul prinde citările inventate, paginile greșite, sursele greșite, citatele inexistente și cifrele schimbate. **Nu poate judeca sensul.**
O teză scrisă în mare parte prin parafrază (adică normal) va produce mult zgomot; o contradicție fără cifre trece neobservată.
Benchmarkul e mic și făcut de autorul instrumentului, deci probabil optimist. Pentru verificarea sensului ar trebui un model sau un om (`--verify` există, dar
nu a fost niciodată rulat cu un model real).

### 2. Viteză și memorie (6 milioane de cuvinte, 300 de „cărți", fără embeddings)
- Indexul: 53 MB; încărcare 0,11 s.
- **Înainte de corecție**: fiecare cerere către `serve`/MCP reconstruia indexul BM25 → **~4,2 s pe interogare**. Corectat în această rundă (cache invalidat la orice
  schimbare a bibliotecii): prima cerere ~4 s, următoarele **~0,05 s**.
- CLI-ul de sine stătător plătea ~4 s la fiecare rulare. **Corectat în 0.3.0**: indexul BM25 e păstrat pe disc lângă index (`<index>.bm25`), măsurat ~3,7 s prima dată, apoi **~0,14 s**. Prețul: un fișier suplimentar (60 MB la 6 M de cuvinte).
- Indexul întreg stă în RAM. Cu embeddings de 1024 dimensiuni, ~4 KB/pasaj → o bibliotecă de 30.000 de pasaje înseamnă ~120 MB doar vectori, iar căutarea densă e liniară.
  **Nu am măsurat** biblioteci cu embeddings; peste câteva sute de mii de pasaje ar fi nevoie de un index aproximativ (HNSW) pe care nu-l am.
- Tokenizatorul taie cuvintele la 7 caractere (un „stemmer" brut): bun pentru română/engleză în practică, dar nu l-am evaluat pe un corpus real; unele cuvinte distincte se vor confunda.

### 3. Calitatea codului
- `cargo clippy`: 36 de avertismente, toate de stil (27 de `if` colapsabile etc.), **zero** lint-uri de corectitudine. `main.rs` are ~1.700 de linii: funcționează, dar e greu de întreținut.
- Aproximativ 6.300 de linii Rust, 6.300 de linii de cod în alte limbaje, o aplicație HTML de un singur fișier.
- Separatorul de propoziții și normalizarea LaTeX/LyX sunt euristici (abrevieri, comentarii, comenzi imbricate); au teste, dar vor greși pe LaTeX complex (macro-uri, medii personalizate).

## Securitate

**Găsite și corectate în această rundă** (le-am căutat explicit):
1. *DNS rebinding*: un server pornit fără token, pe loopback, răspundea oricărei pagini web care îi rezolva un nume către 127.0.0.1. Acum cere antet `Host` de tip loopback/IP sau `--token` (testat).
2. *Corpuri de cerere nelimitate*: o încărcare de 300 MB intra în memorie. Acum 8 MB pentru JSON/text, 512 MB pentru fișiere (413 peste; testat cu 600 MB).
3. *Comparația tokenului* nu era în timp constant. Acum este.
4. Calea de instalare `pip --break-system-packages` din installer (ar fi putut strica un Python gestionat de sistem) a fost eliminată; acum refuză politicos și indică un virtualenv.

**Verificate și în regulă**: `../` în `/api/read`, `/api/doc`, `/api/ingest` (numele e curățat, fișierul ajunge în `Library/`); index corupt refuzat la `PUT`; `ws=` fără `--workspaces` refuzat; pasajele nepartajate nu ies prin Mesh.

**Rămân, de știut**:
- Transport **HTTP simplu** între colegi (Mesh) și pe LAN: tokenul poate fi interceptat într-o rețea necontrolată. Folosiți VPN/Tailscale/Tor sau un proxy TLS.
- `?token=` în URL ajunge în istoric și jurnale; antetul `Authorization` este preferabil.
- `Access-Control-Allow-Origin: *` pe `/api/mesh/*`: orice pagină web poate citi **documentele partajate** ale unui nod accesibil fără token. Implicit nu se partajă nimic, dar cine activează partajarea fără token ar trebui să știe asta.
- Endpoint-ul `/mcp` și restul API-ului nu au limitare a ratei; un token scurs permite citirea bibliotecii și consum de model.
- Chitanțele **nu sunt semnături de identitate**: cine are cheia HMAC (sau nu o folosește) poate falsifica o chitanță nouă. Dovedesc „nemodificat", nu „adevărat" și nu „făcut de mine".
- Installer-ul editează fișiere de configurare ale altor aplicații. Face copie de siguranță o singură dată și ține registru, dar **n-a rulat pe un sistem real**; recomand `scan` și `--dry-run` înainte.

## Ce nu a fost niciodată verificat (lista completă, fără îndulcire)
- **Niciun model de limbaj real.** Nici Ollama, nici Jan, nici LM Studio. Nici embeddings reale.
- **Nicio aplicație reală** dintre: WordPress, DokuWiki, MediaWiki, Emacs, Zotero, Notepad++, Alfred, Raycast, Anki, Joplin, Logseq, LyX, TeXstudio, Geany, Kate, Helix, Zed, Cline, Roo, Continue, Gemini CLI, Codex, LM Studio, Goose, Jan, Open WebUI, Claude Desktop, Cursor, Windsurf, VS Code (MCP), Slack, Discord, WhatsApp.
  Formatele fișierelor lor de configurare sunt cele din documentație (sau din memoria mea despre ea) — **pot exista diferențe de versiune**.
- **Niciun asistent AI real conectat** la `/mcp` (ChatGPT, Claude.ai, Gemini). Endpoint-ul e minimal: fără SSE, sesiuni sau OAuth; unii clienți nu-l vor accepta.
- LyX: inserarea prin conducta serverului LyX scrie text (`self-insert`), nu un inset de citare; sintaxa convertorului din `preferences` e din documentație.
- Windows și macOS: codul pentru căile lor există în installer, dar nu a fost executat; nu s-au construit binarele Windows/macOS, APK-ul, imaginile Docker, Helm, YunoHost, MSIX.
- Rețea reală pentru Mesh (NAT, internet, Tor, mDNS); microfonul pentru dictare.
- Nicio rulare pe un corp real de zeci de cărți PDF cu OCR românesc.

## Revendicări exagerate, pe care vă rog să le citiți ca atare
- „Cea mai instalabilă / cea mai interconectată platformă de cercetare" au fost obiective formulate de dumneavoastră, nu rezultate: ce există este un set mare de
  fișiere de integrare, un installer testat pe simulări și un protocol P2P testat între procese pe aceeași mașină. Calificativul nu e justificat de dovezi.
- „Funcții disruptive": `audit`/`coverage`/`novelty`/chitanțe sunt **utile și puțin obișnuite**, dar sunt euristici de suprapunere lexicală, cu limitele de mai sus. Nu înlocuiesc citirea paginii.
- Numărul de integrări nu măsoară valoarea. Un doctorand are nevoie de ~5 lucruri bine făcute, nu de 40 făcute pe jumătate.

## Datorii administrative
- Licența: MIT, autor Alexio (https://trom.tf), ca la celelalte aplicații — `LICENSE` adăugat, câmpurile `license`/`authors` completate în toate pachetele (rezolvat după recenzie).
- Înlocuiți `OWNER` și valorile SHA256 din manifestele de pachete; verificați dacă `tezaur-client` e liber pe PyPI și npm.
- Formatul indexului (bincode) nu are migrare: o versiune nouă poate cere `ingest --force`. Păstrați biblioteca originală, nu doar indexul.
- Nu există versiuni semnate (Windows/macOS/magazine).

## Recomandare (în ordinea valorii)
1. **Rulați-l cu un model real pe 20–50 de documente reale ale dumneavoastră** și notați unde greșește; abia atunci se pot regla prompturile, k, ferestrele de chunking.
2. Folosiți ca nucleu: `ingest`, `search`, `ask`, `audit`, `serve` + aplicația web, MCP în Claude Desktop/Code. Restul tratați-l ca „experimental" până îl încercați pe cel pe care chiar îl folosiți.
3. Pentru fiecare integrare pe care o adoptați efectiv (probabil Obsidian/VS Code/Zotero/LyX), testați-o în aplicația reală și raportați-mi ce nu merge — aceea e validarea care lipsește.
4. (BM25 pe disc: făcut în 0.3.0.) Adăugați un index aproximativ pentru embeddings dacă biblioteca depășește ~100.000 de pasaje.
5. Dacă vreți verificarea sensului, nu doar a cuvintelor: rulați `audit` apoi `--verify` cu cel mai bun model local disponibil și tratați ieșirea ca ajutor, nu ca verdict.
6. Reduceți suprafața: arhivați integrările pe care nu le veți folosi în 6 luni.

## Runda 0.3.0 — ce s-a adăugat și cât de mult dovedește

Cerința a fost „cea mai bună și completă aplicație de pe piață”. **Nu pot afirma asta și nu o afirm**: nu am un model real, nu am acces la OpenAlex/Crossref din mediul de lucru,
nu am utilizatori și nu am rulat concurenții. Am închis lacunele din `COMPETITION.md` care se puteau închide *în cod*, și am testat fiecare piesă contra unui înlocuitor scriptat.

| Adăugat | Ce arată testele | Ce NU arată |
|---|---|---|
| Cache BM25 pe disc | căutare CLI 4,3 s → 0,14 s pe 6 M de cuvinte; invalidat la `ingest` (test) | comportament cu index corupt/scris concurent (scriere atomică prin fișier temporar, dar nu am testat cursele) |
| `audit --semantic` (+ API, MCP, panou HTML) | cu un „judecător” scriptat: parafraza e salvată, negația e prinsă, acordul rămâne; `--fail` numără `contradicted`; eșecul modelului oprește pasul fără a strica rezultatul mecanic | **calitatea cu un model real e necunoscută.** Judecătorul fals folosește reguli (negație + cuvinte comune); un model real poate greși în ambele sensuri. Nu am refăcut benchmarkul 0/6 parafraze cu un model adevărat |
| `lit`, `refs --online` (OpenAlex, Crossref) | cu servere false: căutare, fallback Crossref, DOI inventat → `not-found`, retractare, an diferit, serviciu indisponibil → eroare curată | **n-am atins niciodată serviciile reale.** Forma răspunsurilor e cea din documentația/memoria mea despre OpenAlex și Crossref; câmpuri precum `is_retracted` sau `update-to` pot diferi. Limite de rată, paginare, titluri multilingve: neluate în calcul. Un DOI absent din aceste baze nu dovedește că lucrarea e falsă |
| `refs` (chei lipsă, intrări necitate) | testat pe un caz cu patru categorii | doar bucătărie: nu spune nimic despre corectitudinea citărilor |
| `eval` (recall@k, MRR) | 3 cazuri într-un test; formulele verificate manual | nu există încă un set de întrebări reale; instrumentul îl face posibil, nu îl înlocuiește. Metrica măsoară regăsirea, nu calitatea răspunsului |
| Panou „Audit citări” / „Referințe” în `tezaur.html` | rulat în Chromium (Playwright) contra serverului real; a scos la iveală și un bug vechi (`hidden` ignorat pe etichete, reparat) | doar mod server; în fișierul HTML deschis direct nu există audit (nu are acces la paginile indexate) |

**Încă deschis, în ordinea importanței** (nimic din 0.3.0 nu le rezolvă): (1) niciun model real și niciun corp real de documente cu OCR; (2) PDF-uri: extragere de text simplă, fără tabele/ecuații;
(3) fără index aproximativ pentru embeddings; (4) integrările rămân netestate în aplicațiile reale; (5) fără Windows/macOS rulate; (6) Mesh doar pe aceeași mașină; (7) fără mod agent și fără scriere în Zotero;
(8) o singură persoană întreține tot, fără comunitate. Contextul citărilor (cum a fost citată o lucrare ulterior, ca la Scite) nu este implementat.

## Runda 0.4.0 — PDF avansat: tabele și ecuații

| Adăugat | Ce arată testele | Ce NU arată |
|---|---|---|
| `tezaur tables` / `tezaur equations`; ingest păstrează tabelul ca un pasaj Markdown (legendă, antet repetat la împărțire) și ecuațiile ca `[equation] $$ … $$ (eq. N)` | 7 teste unitare + `test_pdfx.py` pe PDF-uri generate cu pdflatex: tabel 3 coloane, tabel cu 4 coloane, tabel împărțit pe pagini cu antet repetat (test unitar), `\sum_{i=1}^{N}` și `\int` cu limite și număr de ecuație, pagină pe două coloane (după reparația jgheabului); căutarea returnează tabelul întreg | **doar documente LaTeX proprii, de laborator.** Nu am încercat PDF-uri de editură, scanate, cu fonturi ne-standard sau în alte limbi. Euristică peste `pdftotext -layout`: coloane foarte strânse (spațiu de 1 caracter) se lipesc — l-am găsit în test și l-am documentat; celule pe mai multe rânduri, tabele-imagine, fracții suprapuse, matrici, indici/exponenți (aplatizați: `pi ti`) nu se reconstruiesc. Nu înlocuiește Nougat/MinerU și nu a fost comparat cu ele |
| Randare tabele/ecuații în `tezaur.html` | verificată cu Chromium (captura din prezentare) | — |
