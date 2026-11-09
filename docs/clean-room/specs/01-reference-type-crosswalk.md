# Funcție: Corespondență între tipurile de referință (BibTeX, CSL JSON, RIS)

Sursă publică:
- *BibTeXing* (Oren Patashnik, documentația formatului BibTeX), lista tipurilor de intrare — https://mirrors.ctan.org/biblio/bibtex/base/btxdoc.tex (citit în sesiune prin oglinda https://ipv4.mirrors.cqupt.edu.cn/ctan/biblio/bibtex/base/btxdoc.tex)
- *CSL 1.0.2 Specification*, Appendix III – Types — https://docs.citationstyles.org/en/v1.0.2/specification.html (text citit din sursa rst: https://raw.githubusercontent.com/citation-style-language/documentation/master/specification.rst, licență CC BY-SA 4.0)
- Schema CSL de date de intrare (lista `type`) — https://raw.githubusercontent.com/citation-style-language/schema/master/schemas/input/csl-data.json
- *RIS (file format)*, lista codurilor TY — https://en.wikipedia.org/wiki/RIS_(file_format) (descriere secundară; specificația RIS originală nu a fost citită)

Stare: candidat · Dimensiune: S · Dependențe: niciuna · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie de proiectare a autorului specificației; [necunoscut] = nestabilit.

## Scop
Bibliografia exportată și importată trebuie să păstreze genul lucrării (articol, carte, capitol, teză, raport…). Azi există doar două clase. O singură tabelă de corespondență, folosită de toate importurile și exporturile, evită ca fiecare format să inventeze propria mapare.

## Intrări / ieșiri
- Intrare: un tip de referință într-unul dintre cele trei formate (token BibTeX, valoare CSL `type`, cod RIS `TY`), plus, opțional, un câmp „gen” (de exemplu „teză de master”).
- Ieșire: tipul corespunzător în oricare dintre celelalte două formate și un „tip de origine” păstrat, care permite întoarcerea la tipul exact din formatul de plecare.
- Intrare necunoscută: nu produce eroare; produce tipul generic al fiecărui format (BibTeX `misc`, CSL `document`, RIS `GEN`) [dedus].

## Comportament observabil
- Starea actuală: la export, BibTeX are doar `article` (dacă există titlu de container) sau `misc`; CSL doar `article-journal` sau `document`; RIS doar `JOUR` sau `GEN` (cod: bib, exporturile). Citirea nu reține tipul intrării [observat].
- BibTeX definește 14 tipuri: article, book, booklet, inbook, incollection, inproceedings, conference, manual, mastersthesis, misc, phdthesis, proceedings, techreport, unpublished [observat].
- Orice câmp necunoscut al unei intrări BibTeX este ignorat de BibTeX; un nume de tip necunoscut nu e descris în manual [observat pentru câmpuri; necunoscut pentru tipuri].
- Corespondența de mai jos (BibTeX → CSL → RIS) este decizia specificației [dedus]; fiecare țintă aparține listei publice a formatului ei [observat, verificabil în sursele citate]:
  - article → article-journal → JOUR
  - book → book → BOOK
  - booklet → pamphlet → PAMP
  - inbook, incollection → chapter → CHAP
  - inproceedings, conference → paper-conference → CPAPER
  - manual, techreport → report → RPRT (CSL spune explicit că `report` acoperă și manualele tehnice) [observat pentru manual→report]
  - mastersthesis, phdthesis → thesis → THES, cu „gen” = master / doctorat
  - misc → document → GEN
  - proceedings → book (CSL: `book` e folosit și pentru volume întregi de lucrări de conferință, cu câmpul eveniment) → CONF [observat pentru partea CSL]
  - unpublished → manuscript → UNPB
- Direcția inversă din CSL, pentru tipuri fără echivalent BibTeX exact [dedus]: article-magazine și article-newspaper → article; article (preprint) → misc; webpage, dataset, software, standard, patent, legislation și restul → misc.
- Direcția inversă din RIS [dedus]: JOUR, EJOUR, INPR → article-journal; MGZN → article-magazine; NEWS → article-newspaper; BOOK, EBOOK, EDBOOK → book; CHAP, ECHAP → chapter; CPAPER → paper-conference; CONF → book; RPRT → report; THES → thesis; UNPB, UNPD → manuscript; ELEC, WEB, BLOG → webpage; DATA → dataset; COMP → software; STAND → standard; PAT → patent; GEN, STD și orice alt cod → document.
- Dacă tipul de origine e păstrat, exportul în același format reproduce tipul de origine, nu cel „mediu” (RIS UNPD rămâne UNPD; BibTeX conference rămâne conference) [dedus].
- Distincția master/doctorat se bazează pe câmpul „gen” când există; fără el, teza BibTeX exportată devine `phdthesis` [dedus]; ce face un export RIS din `THES` fără gen este [necunoscut].

## Formate / protocoale
- BibTeX: simbolul după `@`, comparat fără diferență de majuscule [dedus]; manualul nu spune dacă numele tipurilor sunt sensibile la majuscule [necunoscut].
- CSL JSON: șirul din `type`, exact ca în lista Appendix III (de exemplu `article-journal`, `paper-conference`, `legal_case` cu liniuță de subliniere) [observat].
- RIS: două-șase majuscule după `TY  - `; lista citită conține și coduri duplicate semantic (GEN/STD, UNPB/UNPD, WEB/ELEC) [observat în sursa secundară].

## Cazuri limită și erori
- Tip lipsă, gol sau cu majuscule neobișnuite: se tratează ca necunoscut, fără eroare [dedus].
- Cod RIS cu spații la capete: se taie înainte de căutare [dedus].
- Versiunile 2001 și 2011 ale RIS atribuie unele etichete diferit; lista TY poate varia între generatoare [observat în sursa secundară]; care listă folosește un fișier dat este [necunoscut].

## Teste de acceptanță
1. BibTeX `phdthesis` → CSL `thesis` (există în Appendix III) → RIS `THES` (există în lista TY); gen = doctorat.
2. BibTeX `inproceedings` → CSL `paper-conference` → RIS `CPAPER`.
3. CSL `report` cu origine BibTeX `manual`, exportat din nou în BibTeX → `manual`; fără origine → `techreport`.
4. BibTeX `software` (nu e printre cele 14) → CSL `document`, RIS `GEN`, BibTeX `misc`; fără eroare.
5. RIS `ELEC` → CSL `webpage` → BibTeX `misc`; RIS `UNPD` → CSL `manuscript` → înapoi în RIS: `UNPD` când tipul de origine e păstrat, `UNPB` când nu.
6. Proprietate: pentru fiecare tip din intrare, valoarea de ieșire aparține listei publice a formatului țintă (14 BibTeX, lista Appendix III, lista TY citită).

## Necunoscute
- Lista TY oficială (nu doar cea din sursa secundară) și diferențele 2001/2011.
- Comportamentul de majuscule pentru tipuri BibTeX.
- Dacă Appendix III din versiunea 1.0.2 etichetată coincide cu textul de pe ramura principală a depozitului, care a fost citit [necunoscut].

## Verificare (verificare.md)

Spec: 01-reference-type-crosswalk   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
