Aprobare filtru: prin instrucțiunea titularului din conversație (2026-11-09), nu bifată individual; multe surse publice nu au putut fi citite (vezi Necunoscute)

# Index al specificațiilor funcționale (lotul 1, 2026-11-09)

Toate sunt în stare **candidat**: nu sunt aprobate; bifa „Aprobată de mine” din fiecare fișier rămâne nebifată. Sesiune de analiză/specificare, etapa 1 din 2: nu s-a scris și nu s-a modificat cod de produs (`src/`, `variants/` neatinse).

Marcaje folosite în specificații: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie de proiectare; [necunoscut] = nestabilit. Sursele sunt standarde, specificații și documentații publice; `COMPETITION.md` a servit doar ca indiciu de temă, nu s-a copiat comportamentul niciunui produs numit.

| Nr | Slug | Funcție | Stare | Sursă publică principală | Dim. | Depinde de | Ordine |
|----|------|---------|-------|--------------------------|------|------------|--------|
| 01 | [reference-type-crosswalk](01-reference-type-crosswalk.md) | Corespondență tipuri BibTeX / CSL JSON / RIS | implementat (teste trec) | BibTeXing (btxdoc); CSL 1.0.2 App. III; lista RIS TY | S | – | 1 |
| 02 | [persistent-identifiers](02-persistent-identifiers.md) | DOI, ISBN, ISSN, ORCID: extragere, normalizare, validare | implementat (teste trec) | DOI Handbook cap. 2; Crossref (regex DOI); ISBN Users' Manual; ISSN (LoC); ORCID iD | M | – | 2 |
| 04 | [bibtex-person-names](04-bibtex-person-names.md) | Analiza numelor de persoane (First/von/Last/Jr) | implementat (teste trec) | BibTeXing (btxdoc); CSL 1.0.2 „Name Particles” | M | – | 3 |
| 03 | [bibtex-values-and-types](03-bibtex-values-and-types.md) | Citire BibTeX completă: @string, `#`, tip, crossref | parțial: avertismentul de câmpuri obligatorii doar pentru `article` (INTREBARI 5) | BibTeXing (btxdoc) | M | 01, 04 | 4 |
| 05 | [ris-import](05-ris-import.md) | Import RIS | implementat (teste trec) | RIS (file format), descriere publică secundară | M | 01, 02, 04 | 5 |
| 06 | [csl-json-import](06-csl-json-import.md) | Import CSL JSON | implementat (teste trec) | CSL 1.0.2 + schema `csl-data.json` (MIT) | M | 01, 02, 04 | 6 |
| 07 | [duplicate-by-identifier](07-duplicate-by-identifier.md) | Duplicate bibliografice după DOI / ISBN | implementat (teste trec) | DOI Handbook §2.4; ISBN Users' Manual; CSL App. III | S | 01, 02 | 7 |
| 12 | [crossref-client-conduct](12-crossref-client-conduct.md) | Identificare, limite de rată, repetări, cache, retractări, agenție | implementat (teste trec) | Crossref REST API (acces + `rest-api-doc`); DOI Handbook §2.5 | M | 02 | 8 |
| 13 | [oai-pmh-harvest](13-oai-pmh-harvest.md) | Recoltare OAI-PMH / Dublin Core din arhive instituționale | parțial: câmpurile DC description/subject/language ignorate (INTREBARI 13) | OAI-PMH 2.0 | L | 01, 02, 12 | 9 |
| 08 | [citation-locators](08-citation-locators.md) | Localizatori de citare: pagină, capitol, interval, cifre romane | implementat (teste trec) | CSL 1.0.2 (lista Locators, intervale) + fișierele de localizare CSL | M | – | 10 |
| 11 | [epub-page-list](11-epub-page-list.md) | Pagini tipărite pentru EPUB (page-list, pagebreak) | parțial: etichetele de pagină nearabe nu se păstrează în index; ediția tipărită neidentificată (INTREBARI 17–18) | EPUB 3.3 (W3C) | L | 08, 09 | 11 |
| 09 | [unicode-index-text](09-unicode-index-text.md) | Normalizare, pliere, segmentare Unicode pentru indexare | implementat (teste trec) | UAX #15; UAX #29 | M | – | 12 |
| 10 | [phrase-query](10-phrase-query.md) | Căutare de expresie exactă și apropiere | implementat (teste trec); implicit oprit, se activează cu `--phrases` | Introduction to Information Retrieval, cap. 2.4 | L | 09 | 13 |
| 14 | [receipt-integrity](14-receipt-integrity.md) | Chitanțe: JSON canonic (JCS) și identitate SHA-256 | implementat (teste trec) | RFC 8785 (+ FIPS 180-4, RFC 4231 necitite) | M | – | 14 |
| 15 | [ui-accessibility](15-ui-accessibility.md) | Cerințe WCAG 2.2 A/AA pentru interfața web | parțial: implementate verificările statice (nume, tastatură, regiuni de stare, contrast, țintă, lang); nu s-au putut verifica reflow 320 px, cititor de ecran, mărime randată (INTREBARI 25–26) | WCAG 2.2 (criterii de succes) | M | – | 15 |

Notă despre numerotare: numărul din fișier (Nr) urmează ordinea de redactare; coloana „Ordine” este ordinea recomandată de implementare (dependențele înaintea celor care le folosesc, apoi după valoare pentru utilizator). Comportamentul care există deja (BM25, fuziune de ranguri RRF, cosinus, audit mecanic, chitanțe HMAC, export RIS/CSL/BibTeX, OCR, tabele PDF, FFF) nu a fost respecificat.

## De unde provine fiecare temă (doar ca indiciu)

| Nr | Indiciu din repo |
|----|------------------|
| 01 | README („bib --to ris / csl-json / bibtex”); codul de export folosește două clase de tip |
| 02 | REVIEW.md (verificare online, DOI inventat → not-found); codul păstrează doar DOI |
| 03, 04 | README („BibTeX … Better BibTeX .bib”); citirea actuală aruncă tipul și ignoră @string / crossref |
| 05, 06 | COMPETITION.md (ecosistemul de gestionare a referințelor, ca temă); export fără import |
| 07 | cod: bibliografia din mai multe fișiere fără detectare de duplicate |
| 08 | README (citări `[@cheie, p. 3]`, audit); cod: orice număr după cheie e pagină |
| 09 | README / REVIEW (diacritice românești, ligaturi din PDF); cod: tabelă de pliere manuală |
| 10 | README (căutare BM25; „audit” verifică citate word-for-word, dar căutarea nu are expresii) |
| 11 | README („EPUB/DOCX have no page numbers”) |
| 12 | REVIEW.md („Limite de rată, paginare, titluri multilingve: neluate în calcul”) |
| 13 | COMPETITION.md §3 („Descoperire de literatură” ca lipsă) |
| 14 | README / REVIEW (chitanțe: „tamper evidence”; amprentă de document pe 64 de biți) |
| 15 | COMPETITION.md §3 („Interfață lustruită”); cerința de a respecta WCAG pentru interfață |

Înregistrări suplimentare: `SKIPPED.md` (teme neluate și motivul), `JURNAL.md` (sursele citite, licențe, metodă).
