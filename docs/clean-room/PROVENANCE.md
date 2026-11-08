# Proveniență și clean-room — TEZAUR (chiuta/TEZAUR)

Document de completat cu fapte reale. Nu declara clean-room pentru ce nu s-a întâmplat așa.

## Ce a existat până acum (completează onest)
- Ținta/țintele: (fără țintă fixă; completează doar dacă analizezi un produs terț)
- Cum s-a documentat comportamentul țintei (documentație publică, utilizare, formate observate, decompilare?): ______
- A fost folosit vreun cod, pseudocod, decompilare sau asset al țintei în sesiunile de dezvoltare? DA / NU / NU ȘTIU: ______
- Sesiunile de dezvoltare au avut vreodată acces la binarul sau codul țintei? ______

## Risc de licență
Dacă nu analizezi niciun produs terț, folosește doar PROVENANCE.md pentru documentarea surselor și dependențelor.

## Dependențe terțe
Rulează `cargo license` (sau `cargo deny check licenses`) și atașează rezultatul. Verifică compatibilitatea cu licența proiectului.

## Pentru lucrul de acum înainte
Folosește fluxul din acest kit (1-analiza → 2-filtru → 3-implementare). Orice funcție nouă inspirată de o țintă trece prin specificație și filtru.

## Recomandare
Pentru lansări comerciale sau donații semnificative, cere o opinie de la un avocat de proprietate intelectuală și fă căutare de mărci (EUIPO), cum ai făcut pentru OSATURA.

## Constatări din audit adversarial (2026-11-08) — fapte verificate în repo

**Corectură:** o versiune anterioară spunea că nu există nicio mențiune a unei ținte terțe în sursă. Este inexact: README-ul și `COMPETITION.md` citează produse studiate (vezi mai jos).

### Istoric
- Primul commit (`820070e`, 2026-10-29, „TEZAUR 0.3.0") adaugă 283 de fișiere și 20.230 de linii. Documentele menționează versiuni anterioare (0.1.0, 0.2.x), dar Git nu le conține: **proveniența dinainte de 0.3.0 nu poate fi reconstituită din repo.**
- Autori: `Alexio <alexio@trom.tf>` (89 de commit-uri) și două commit-uri din interfața GitHub (`Alexandru Chiuta`, CNAME). Nu există contribuții externe. Nu există fișiere șterse.
- Commit-urile poartă `Co-Authored-By: Claude`: cod scris în mare parte cu un model AI; eventualele influențe din datele de antrenare nu pot fi excluse.

### Proveniență și inspirație (de declarat explicit)
- `README.md:219-223` preia idei de la **UberStudent** (distribuție Linux pentru studenți, abandonată): meniu pe abilități academice, ajutoare de studiu, ghid de prim start. Doar idei de structură; nu s-a găsit text sau cod copiat.
- `COMPETITION.md` compară cu alte produse (AnythingLLM, GPT4All, Kotaemon, Khoj, NotebookLM, PrivateGPT, Open WebUI ș.a.), din surse citate. Documentul spune că niciun concurent nu a fost rulat. Nu s-a găsit text copiat din concurenți.
- Implementări proprii verificate: SHA-256/HMAC (`src/sha256.rs`, constante standard FIPS 180-4), cititor zip/docx/epub în `tezaur.html`. Nu există liste de stopwords preluate.
- Nu există antete de copyright în fișiere individuale; acoperirea e globală prin `REUSE.toml` (MIT, „2026 Alexio").

### Afirmații din documentație mai tari decât dovezile
- „Răspunde doar din biblioteca ta" (`index.html`, `README.md`): `REVIEW.md` spune că proiectul **nu a fost rulat niciodată cu un model real**, iar README admite că verificarea citărilor e mecanică. Formulare apărabilă: „instruit să răspundă din sursele tale; citările sunt verificate mecanic". Evită „garantează" până la un test cu model real.
- „100% local / funcționează offline": aplicația web încarcă pdf.js de pe cdnjs, transformers.js de pe jsdelivr (+ modelul Whisper de pe Hugging Face), iar boții trec mesaje prin servicii terțe. Precizează în deck, README și PWA.
- „Binar unic, fără dependențe": PDF-urile avansate și OCR cer `pdftotext`, `pdftoppm`, `tesseract`.
- `COMPETITION.md` descrie Open WebUI ca BSD-3 „de verificat"; sursele publice indică acum o licență proprie cu clauză de branding. Corectează sau marchează „neverificat".
- Captura etichetată „Zotero real încarcă pluginul" arată un editor de cod, iar textul spune „netestat în Zotero". Aliniază eticheta.

### Licențe și distribuire
- Cargo: 170 de pachete, toate permisive (MIT/Apache-2.0/ISC/BSD/Zlib/Unicode/Unlicense/CDLA-Permissive); `r-efi` are alternativă permisivă; `ring` cere păstrarea ambelor notițe (Apache-2.0 AND ISC).
- Pachetele (arhive, `.deb`, PortableApps) includ doar `LICENSE`-ul propriu, fără notițele dependențelor. Recomandare: `cargo about` și un `THIRD-PARTY-NOTICES` în toate pachetele; majoritatea variantelor (extensie, VS Code, Electron, Tauri etc.) nu includ textul licenței.
- JS încărcat de pe CDN fără `integrity=` (0 apariții): `tezaur.html:223, 827`, `pwa/index.html:202`, extensia, YunoHost, Logseq. Android expune `AndroidBridge` în aceeași pagină și permite trafic necriptat. Recomandare: vendorizează cu notița de licență sau adaugă SRI; pdf.js local în APK și în extensie.
- `OWNER/tezaur` (placeholder) apare în 22 de fișiere de distribuție (Homebrew, Scoop, winget, AUR, Flatpak, Snap, F-Droid, Helm etc.), multe cu sha256 de zerouri. Înlocuiește înainte de orice publicare.
- Modelele (`llama3.1:8b`, `bge-m3`) sunt doar recomandate, nu incluse; licența Llama 3.1 are restricții proprii și nu e menționată. Licența lui `Xenova/whisper-tiny` nu a putut fi verificată (acces blocat în audit).
- `Romanian.isl` e descărcat în CI din directorul „Unofficial" al Inno Setup; licența lui nu a fost verificată. Alte descărcări de pe `main`/`latest` fără versiuni fixate.
- Imaginea Docker instalează `poppler-utils` (GPL) și `tesseract-ocr` ca procese externe; distribuirea imaginii aduce obligațiile pachetelor Debian.

### Confidențialitate (observații)
Ieșiri din rețea din binar doar la `lit` și `refs --online` (OpenAlex, Crossref); fără telemetrie găsită. Dictarea nativă din browser poate trimite sunetul la un serviciu cloud (avertizat în UI).

### De completat de către titular (nu pot fi deduse din repo)
- Ce a precedat 0.3.0 și cum a fost dezvoltat: ______
- Iconițele („T") au fost desenate de tine sau generate cu un instrument/font? ______
- Ai verificat licențele: Whisper tiny (Xenova), Romanian.isl, Llama 3.1 (dacă îl recomanzi)? ______

**Limite:** nu s-au rulat build-uri sau `cargo deny`; licențele npm/Gradle tranzitive nu au fost verificate; nu s-a făcut comparare de similaritate cu cod terț. Nu este opinie juridică.
