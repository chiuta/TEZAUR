# Funcție: Import de bibliografie în format CSL JSON

Sursă publică:
- *CSL 1.0.2 Specification* (variabile, tipuri, nume, date), licență CC BY-SA 4.0 — https://docs.citationstyles.org/en/v1.0.2/specification.html ; text citit din https://raw.githubusercontent.com/citation-style-language/documentation/master/specification.rst
- Schema JSON „CSL input data” (licență MIT, fișierul LICENSE.txt al depozitului) — https://raw.githubusercontent.com/citation-style-language/schema/master/schemas/input/csl-data.json ; id-ul schemei: https://resource.citationstyles.org/schema/v1.0/input/json/csl-data.json

Stare: candidat · Dimensiune: M · Dependențe: 01 (tipuri), 02 (identificatori), 04 (nume) · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Programul își scrie bibliografia în CSL JSON (și în plicul `citation/csl` al lanțului de aplicații proprii), dar nu o poate citi. Orice gestionar de referințe poate exporta CSL JSON; fără import, utilizatorul trebuie să treacă prin BibTeX, cu pierderi de câmpuri.

## Intrări / ieșiri
- Intrare: un fișier JSON care conține o listă de elemente CSL, fie ca listă de nivel superior, fie ca `payload` al unui plic cu `type` = `citation/csl` (plicul propriu al lanțului de aplicații, definit într-un depozit separat) [observat pentru structura plicului scris de program (cod: modulul fff); definiția plicului în sine nu este un standard public].
- Ieșire: intrări bibliografice în aceeași formă ca la BibTeX (spec 03) și RIS (spec 05), plus avertismente.

## Comportament observabil
- Starea actuală: doar export; nu există cititor CSL JSON (cod) [observat].
- Schema cere la fiecare element doar `type` și `id`; `id` poate fi șir sau număr; proprietățile necunoscute sunt interzise de schemă [observat]. Cititorul este tolerant: ignoră ce nu cunoaște, fără eroare [dedus].
- `type` este unul din valorile din Appendix III / enumerarea schemei (45 de valori, de la `article` la `webpage`) [observat]; trece prin corespondența din spec 01. Un tip necunoscut devine tip generic cu avertisment [dedus].
- Cheia intrării: `citation-key` dacă există (câmp prezent în schemă), altfel `id` transformat în șir [observat pentru câmpuri; ordinea de preferință dedusă].
- Nume (`author`, `editor`, `translator`, `container-author` ș.a.): listă de obiecte; părți: `family`, `given`, `dropping-particle`, `non-dropping-particle`, `suffix`, `comma-suffix`, `static-ordering`, `literal`, `parse-names` [observat]. `literal` este un nume nedivizat (instituție). Semnificația exactă a `parse-names`, `static-ordering` și `comma-suffix` nu reiese din schemă [necunoscut].
- Un autor devine textul „Nume, Prenume” (particula non-dropping se prepune la nume, cea dropping se adaugă după prenume, sufixul după virgulă), apoi se unește prin ` and `; un `literal` se pune între acolade ca nume nedivizat, ca în spec 04 [dedus].
- Dată (`issued`, `accessed`, `event-date`, `original-date`, `submitted`, `available-date`): obiect cu `date-parts` (listă de una sau două liste de câte 1–3 elemente, șiruri sau numere), `season`, `circa`, `literal`, `raw` [observat]. Două liste în `date-parts` înseamnă interval [dedus pe baza „minItems 1, maxItems 2” și a secțiunii despre intervale de date].
- Descrierea schemei vorbește despre un șir EDTF ca reprezentare preferată, dar definiția efectivă permite doar obiectul [observat]; cititorul acceptă și un șir, din care ia primele patru cifre [dedus; necunoscut dacă fișierele reale au așa ceva].
- Anul = primul element din prima listă din `date-parts`; dacă lipsește, primele patru cifre consecutive din `raw` sau `literal` [dedus]. Luna și ziua nu sunt reținute de intrarea actuală (are doar an) [observat în cod].
- Câmpuri mapate [dedus]: `title` → titlu; `container-title` → container; `DOI` → doi (spec 02); `ISBN`, `ISSN` → isbn, issn (spec 02; pot conține mai multe valori într-un șir); `URL` → url; `volume`, `issue`, `page`, `publisher`, `publisher-place`, `edition`, `abstract`, `language`, `genre`, `keyword`, `note` → câmpurile omonime; numerele pot fi șiruri sau numere.
- `locator` și `citation-number`, `citation-label`, `year-suffix` sunt variabile de randare, nu date bibliografice; se ignoră [dedus]; `locator` ar trebui să fie însoțit de o etichetă de tip de locator [observat în specificație].
- Variabila `custom` (proprietăți arbitrare) se ignoră [dedus].
- Ordinea elementelor din fișier se păstrează [dedus].

## Formate / protocoale
- JSON UTF-8. Rădăcina: listă de obiecte, sau obiect-plic cu `payload` listă. Un obiect de rădăcină fără aceste forme este eroare de format [dedus].
- Numerele mari (`id` numeric) se convertesc în șir fără pierdere până la 2^53 [dedus].

## Cazuri limită și erori
- JSON invalid: eroare clară cu poziția aproximativă; nicio intrare nu se importă [dedus].
- Element fără `type` sau fără `id`: se sare cu avertisment (încalcă schema) [observat pentru cerință; comportamentul dedus].
- Două elemente cu aceeași cheie: se păstrează primul, se avertizează [dedus].
- `author` care nu e listă, sau element de listă care nu e obiect: se sare numele cu avertisment [dedus].
- `date-parts` cu `null` sau gol: an necunoscut, intrarea rămâne [dedus].

## Teste de acceptanță
1. `[{"id":"a","type":"article-journal","title":"T","author":[{"family":"Smith","given":"Jane"}],"issued":{"date-parts":[[2019,5,3]]},"DOI":"10.1/x","container-title":"J"}]` → cheie `a`, tip articol de revistă, autor „Smith, Jane”, an 2019, doi `10.1/x`, container „J”.
2. `"issued":{"raw":"May 2019"}` → an 2019; `"issued":{"literal":"n.d."}` → an necunoscut, fără eroare.
3. `"author":[{"literal":"Council of Europe"}]` → un singur nume nedivizat, citat întreg.
4. `"author":[{"family":"Gogh","given":"Vincent","non-dropping-particle":"van"}]` → nume afișat „van Gogh” (particula rămâne).
5. Element fără `id` → sărit cu avertisment; celelalte elemente se importă.
6. Plic `{"fff":2,"type":"citation/csl","payload":[…]}` produs de export → aceleași intrări ca din lista goală de nivel superior; un export urmat de import păstrează cheie, titlu, autori, an, doi, url.
7. `"id":42` numeric → cheia „42”.
8. `"type":"inventat"` → tip generic și avertisment.

## Necunoscute
- Semnificația `parse-names`, `static-ordering`, `comma-suffix` în datele reale.
- Dacă exporturile reale folosesc șiruri EDTF pentru date.
- Licența fișierelor de localizare nu intră aici; doar schema (MIT) și specificația (CC BY-SA 4.0) au fost citite.
- Versiunea schemei citite (ramura principală) față de cea legată de CSL 1.0.2 [necunoscut].

## Verificare (verificare.md)

Spec: 06-csl-json-import   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [x] Licența/EULA țintei verificată și notată în jurnal (licențele surselor citite sunt notate în JURNAL.md (fișierele citite: verificate în depozit))
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
