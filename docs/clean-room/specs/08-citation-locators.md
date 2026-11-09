# Funcție: Localizatori de citare (pagină, capitol, secțiune, interval) în textul de verificat

Sursă publică:
- *CSL 1.0.2 Specification*: lista „Locators” din Appendix II, variabila `locator` (Appendix IV), secțiunile „Range Delimiters” și „Page Ranges” — https://docs.citationstyles.org/en/v1.0.2/specification.html ; text citit din https://raw.githubusercontent.com/citation-style-language/documentation/master/specification.rst (CC BY-SA 4.0)
- Fișierele de localizare CSL (termenii și abrevierile pentru en-US, ro-RO, fr-FR): https://raw.githubusercontent.com/citation-style-language/locales/master/locales-en-US.xml , …/locales-ro-RO.xml , …/locales-fr-FR.xml (doar cuvintele-termen au fost folosite; licența fișierelor nu a fost verificată)

Stare: candidat · Dimensiune: M · Dependențe: niciuna (11 depinde de ea) · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Verificarea citărilor `[@cheie, p. 3]` compară propoziția cu pagina citată. Azi orice număr găsit după cheie este luat drept pagină, deci „cap. 2” sau „[@cheie, 2019]” produc verdicte false, iar intervalele și cifrele romane se pierd.

## Intrări / ieșiri
- Intrare: textul dintre cheie și sfârșitul citării (sau `;`), ex. „, pp. 3–5”, „, cap. 2”, „, p. xii”.
- Ieșire: o listă de localizatori; fiecare are o etichetă (din lista CSL), o valoare (număr, număr roman, alfanumeric) sau un interval (de la, până la), și un indicator „verificabil ca pagină”.
- Exemplu: „, pp. 321–28” → eticheta pagină, interval 321–328, verificabil ca pagină.

## Comportament observabil
- Starea actuală: după cheie se caută prima secvență de cifre din text și devine pagină, indiferent de etichetă; intervalele păstrează doar începutul; numerele romane și valorile fără cifre dau „fără pagină”; un an după cheie devine pagină și poate declanșa „pagină în afara documentului” (cod: modulul de audit, analiza grupului de citări) [observat]. Verificarea răspunsurilor modelului recunoaște doar „p. ” urmat de cifre (cod: modulul de prompt) [observat].
- Specificația CSL definește 30 de tipuri de locator: act, appendix, article-locator, book, canon, chapter, column, elocation, equation, figure, folio, issue, line, note, opus, page, paragraph, part, rule, scene, section, sub-verbo, supplement, table, timestamp, title-locator, verse, version, volume [observat]. Variabila `locator` trebuie însoțită de o etichetă de tip [observat].
- Etichetele scrise de oameni se aduc la tipul CSL prin termenii din fișierele de localizare: en-US: p./pp. (page), chap./chaps. (chapter), sec./secs. (section), para./paras. (paragraph), fig./figs. (figure), tbl./tbls. (table), vol./vols. (volume), l./ll. (line), n./nn. (note), simbolurile § (section) și ¶ (paragraph); ro-RO: p./pp., cap., sec., par., fig., tbl., vol., n., § și ¶, cu formele lungi pagina/paginile, capitolul, secțiunea, paragraful, figura, tabelul, volumul, nota; fr-FR: p., chap., sect., paragr., fig., tab., vol., n., § [observat]. Forme suplimentare uzuale („pag.”, „f.”, „ff.”) nu sunt în fișierele citite [necunoscut]; pot fi adăugate ca tolerare [dedus].
- Potrivirea etichetelor ignoră majusculele și punctul final [dedus]. Eticheta poate lipsi [observat că CSL o cere în date, dar textul scris de oameni o omite]; un număr fără etichetă se consideră pagină [dedus].
- Interval: separat prin cratimă, liniuță en (–) sau două cratime; CSL randează cratimele din locator ca liniuță en [observat]. Forma prescurtată a capătului final se extinde: „321–28” înseamnă 321–328; „321–8” înseamnă 321–328 în formatul minimal [observat din exemplele formatelor de intervale; regula generală de extindere este dedusă].
- Listă de valori într-un locator („3, 5” sau „3 și 5” / „3 and 5”): fiecare element e o pagină separată [dedus]. Punctul și virgula `;` rămâne separatorul dintre citări diferite din același grup [observat în cod, modulul de audit].
- Cifre romane (i, xii, XIV) sunt valori valide de pagină (paginile prefeței); se păstrează ca etichetă de pagină, fără valoare numerică, și nu sunt comparate cu numărul de pagini ale documentului [dedus].
- Verificabil ca pagină = eticheta este `page` (sau lipsește) și valoarea este numerică sau interval numeric. Pentru toate celelalte etichete (capitol, secțiune, figură…) citarea rămâne în raport cu eticheta, dar nu se compară cu o pagină și nu produce „pagină greșită” [dedus].
- Interval verificat: propoziția e susținută dacă oricare pagină din interval o susține; „pagină în afara documentului” dacă sfârșitul intervalului depășește numărul de pagini [dedus].
- Un număr fără etichetă care coincide cu anul lucrării citate (de exemplu `[@smith2019, 2019]`) nu se tratează ca pagină [dedus]; ce alte numere fără etichetă ar putea fi ani sau volume este [necunoscut].
- „ff.” și „et seq.” după o pagină (de la pagina n încolo): pagina n devine start, sfârșitul nedeterminat; se verifică doar pagina n [dedus; sursele citite nu le descriu — necunoscut].
- Pentru documentele fără pagini (text, DOCX): localizatorii se păstrează, nu se verifică (ca azi pentru absența paginilor) [observat în cod].

## Formate / protocoale
- Grupul de citări rămâne `[@cheie1, localizator; @cheie2, localizator]`; funcția doar înlocuiește interpretarea restului după cheie. [dedus]
- Caractere: cratimă `-`, liniuță en U+2013, liniuță em U+2014 acceptate ca separator de interval [dedus].

## Cazuri limită și erori
- Localizator gol („[@cheie,]”): nicio valoare; citarea rămâne validă [dedus].
- Interval inversat („5–3”): raportat ca interval greșit, nu ca pagină în afara documentului [dedus].
- Valoare neinterpretabilă („p. ???”): păstrată ca text, nu verificabilă [dedus].
- Etichetă necunoscută („voi. 2”): considerată text, citarea e valabilă, fără verificare de pagină [dedus].

## Teste de acceptanță
1. `[@a, p. 3]` → pagină 3 verificabilă (comportament neschimbat).
2. `[@a, cap. 2]` și `[@a, chap. 2]` → etichetă capitol, valoare 2, neverificabil ca pagină; nu apare „pagină în afara documentului” pentru un document de 1 pagină.
3. `[@a, pp. 321–28]` → interval 321–328; `[@a, pp. 3-5]` → interval 3–5; susținut dacă oricare din paginile 3, 4, 5 susține propoziția.
4. `[@a, p. xii]` → pagină romană „xii”, neverificabilă numeric, fără eroare.
5. `[@a, 2019]` cu `@a` publicat în 2019 → fără localizator de pagină; fără „pagină în afara documentului”.
6. `[@a, § 4]` → secțiune 4; `[@a, fig. 2]` → figură 2.
7. `[@a, p. 3, 5]` → paginile 3 și 5, fiecare verificabilă.
8. Un document de 10 pagini și `[@a, pp. 9–12]` → sfârșitul 12 > 10, raport „în afara documentului”.

## Necunoscute
- Regula completă de extindere a capetelor abreviate de interval (în afara exemplelor).
- Forme abreviate românești nelistate („pag.”, „f.”) și cum scriu utilizatorii reali.
- Tratamentul numerelor fără etichetă diferite de pagină.
- Dacă fișierele de localizare au aceeași licență ca specificația (aici sunt folosite doar cuvintele-termen).

## Verificare (verificare.md)

Spec: 08-citation-locators   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
