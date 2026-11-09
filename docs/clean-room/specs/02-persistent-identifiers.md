# Funcție: Identificatori persistenți (DOI, ISBN, ISSN, ORCID): extragere, normalizare, validare

Sursă publică:
- *DOI Handbook*, cap. 2 „Numbering” (sintaxă, sensibilitate la majuscule, prezentare, codificare în URL) — https://www.doi.org/doi-handbook/ ; pagina citită: https://albany.doi.org/doi_handbook/2_Numbering.html
- Crossref, *DOIs and matching regular expressions* — https://www.crossref.org/blog/dois-and-matching-regular-expressions/ (citit prin oglindă: https://www-crossref-org.turing.library.northwestern.edu/blog/dois-and-matching-regular-expressions)
- *ISBN Users' Manual*, ed. a 7-a, Agenția Internațională ISBN — https://www.nb.no/content/uploads/2018/04/ISBN-International-Users-Manual-7th-edition_absolutely_final.pdf (ISO 2108)
- Biblioteca Congresului, *Calculating the check digit* (ISSN, ISO 3297) — https://www.loc.gov/issn/check.html
- ORCID, *Format of the ORCID iD* (ISO/IEC 7064 MOD 11-2) — https://support.orcid.org/hc/articles/360006897674

Stare: candidat · Dimensiune: M · Dependențe: niciuna · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Referințele din teză se potrivesc, se deduplică și se verifică online prin identificatori. Azi un DOI e curățat doar de trei prefixe fixe, iar ISBN, ISSN și ORCID nu sunt reținuți deloc. Funcția dă o formă canonică și o validare, folosite de import, deduplicare și verificarea online.

## Intrări / ieșiri
- Intrare: un text liber (câmp de intrare bibliografică, URL lipit, paragraf) sau o valoare deja izolată.
- Ieșire pentru fiecare identificator găsit: tipul (doi, isbn13, isbn10, issn, orcid), forma canonică, forma de afișat, verdictul (valid, sumă de control greșită, structură greșită) și poziția în text.
- Exemplu: `https://doi.org/10.1006/JMBI.1998.2354` → tip doi, canonic `10.1006/jmbi.1998.2354`, afișat `10.1006/JMBI.1998.2354`.

## Comportament observabil

DOI
- Starea actuală: se taie doar prefixele exacte `https://doi.org/`, `http://doi.org/` și `doi:` (litere mici), nimic altceva; comparația de egalitate e sensibilă la majuscule; în adresele către serviciile online DOI-ul intră netranscris (cod: modulul de căutări online) [observat].
- Un nume DOI are prefix (începe cu `10`, punct, cod de înregistrare; codul poate fi subdivizat prin puncte, ca în `10.1000.10`), bară oblică și sufix ales de înregistrant [observat, Handbook §2.2.1–2.2.3].
- Sufixul poate conține orice caracter grafic tipărabil din Unicode [observat].
- Nume DOI nu fac distincție între majuscule și minuscule, cu pliere de majuscule doar pentru ASCII [observat, §2.4]. Deci: cheia de comparație = forma cu litere ASCII mici; formele non-ASCII nu se pliază; la afișare se păstrează grafia originală [dedus pe baza observației].
- Eticheta `doi:` nu face parte din numele DOI; forma recomandată de URL este `https://doi.org/` + nume [observat, §2.6.1–2.6.2]. Etichetele recunoscute la intrare: `doi:` în orice grafie de majuscule, `https://doi.org/`, `http://doi.org/`, `https://dx.doi.org/`, `http://dx.doi.org/` [dedus; gazda veche dx nu e în paginile citite — necunoscut].
- În URL, `#` trebuie codificat `%23`, `?` ca `%3F`, `%` ca `%25`; codificarea ține de reprezentarea URL, DOI-ul în sine rămâne necodificat [observat, §2.5]. La intrare dintr-un URL, secvențele `%xx` se decodifică înainte de validare [dedus].
- Extragerea dintr-un text: modelul recomandat de Crossref pentru DOI moderne este `10.` + 4–9 cifre + `/` + unul sau mai multe caractere din: litere, cifre, `-` `.` `_` `;` `(` `)` `/` `:`, fără diferență de majuscule; acoperă ≈99% din 74,9 milioane de DOI [observat]. Restul (≈1%) conține și caractere ca `<`, `>`, `&` și au modele separate; Crossref spune că acoperirea lor completă cere o duzină de modele în plus și scade precizia [observat].
- Punctuația finală: un punct, două puncte, punct și virgulă, cratimă sau liniuță de subliniere la capăt poate fi parte din DOI sau din propoziție; Crossref semnalează problema fără soluție [observat]. Decizie: la extragerea din text liber se taie la capăt `.` `,` `;` `:` și paranteza de închidere dacă nu are pereche în DOI; la valoare izolată (câmp `doi`) nu se taie nimic [dedus]. Dacă DOI-ul real se termină cu un asemenea caracter, e [necunoscut] cum se recuperează; se păstrează forma lungă ca „candidat alternativ” [dedus].

ISBN
- ISBN-13 are 13 cifre: prefix GS1 (978 sau 979), grup de înregistrare, înregistrant, publicație, cifră de control [observat].
- Cifra de control: primele 12 cifre se înmulțesc alternativ cu 1 și 3, începând cu 1; suma modulo 10; control = 10 minus restul, iar 10 devine 0; un număr e valid dacă suma ponderată completă (cu cifra de control) se împarte exact la 10 [observat].
- Cratimele și spațiile nu au semnificație lexicală, sunt doar pentru lizibilitate, și nu fac parte din număr; metadatele cer forma de 13 cifre fără ele [observat]. Eticheta „ISBN” precede numărul tipărit [observat]; la intrare se acceptă cu sau fără etichetă și cu `:` după ea [dedus].
- Forma canonică: 13 cifre fără separatori. Gruparea cu cratime la afișare cere tabela de intervale a Agenției ISBN, necitită: afișare doar fără cratime [dedus; necunoscut pentru intervale].
- ISBN-10: 10 caractere, ultimul cifră sau X, modulo 11 cu ponderi 10…1; conversia în ISBN-13 = prefix 978 + primele 9 cifre + control nou [dedus din cunoștințe generale; pagina citită declară că metoda ISBN-10 nu e inclusă — necunoscut]. Un ISBN-10 cu sumă greșită se raportează „sumă greșită”, nu se corectează.

ISSN
- Două grupe de patru caractere unite prin cratimă; primele șapte cifre, al optulea cifră sau X [observat din exemplul și descrierea citite].
- Control: cele șapte cifre se înmulțesc cu 8, 7, 6, 5, 4, 3, 2; suma modulo 11; control = 11 minus restul, 10 se scrie X, restul 0 dă 0 [observat].
- Forma canonică: `NNNN-NNNX` cu X majusculă [dedus].

ORCID
- 16 caractere în patru grupe de câte patru cu cratimă; ultimul caracter e cifră sau X; toate 16 sunt obligatorii, zerourile de la început nu se omit [observat].
- Control (ISO/IEC 7064 MOD 11-2): pentru fiecare cifră din cele 15 de bază, total = (total + cifră) × 2; rest = total modulo 11; rezultat = (12 − rest) modulo 11; 10 se scrie X [observat].
- Afișare: URI complet `https://orcid.org/` + iD; minimul cerut e URI-ul complet [observat]. La intrare se acceptă URI-ul complet sau iD-ul gol; `x` mic se transformă în `X` [dedus].

## Formate / protocoale
- Toate comparațiile se fac pe forma canonică; ieșirile structurate poartă ambele forme (canonică și originală). [dedus]
- Câmpurile noi ale intrării bibliografice: doi, isbn, issn, orcid (un autor poate avea ORCID propriu; asocierea ORCID–autor în BibTeX/RIS/CSL JSON nu e standardizată în sursele citite) [necunoscut].

## Cazuri limită și erori
- Mai mulți identificatori în aceeași valoare (ISBN-10 și ISBN-13 ale aceleiași cărți): se returnează toți, fiecare cu verdictul lui [dedus].
- Cifră de control greșită: verdict „sumă greșită”, valoarea se păstrează; nu se aruncă eroare [dedus].
- DOI într-un URL cu fragment (`#sectiune`): fragmentul de la `#` necodificat se ignoră, `%23` se păstrează ca `#` [dedus].
- DOI cu caractere non-ASCII: acceptat; nu se pliază majusculele non-ASCII [observat prin §2.4, aplicare dedusă].
- Un DOI valid ca sintaxă nu e dovada că există; existența se verifică doar online (spec 12) [dedus].

## Teste de acceptanță
1. `https://doi.org/10.1006/JMBI.1998.2354` și `doi:10.1006/jmbi.1998.2354` dau aceeași cheie canonică `10.1006/jmbi.1998.2354`.
2. `10.1000.10/123` este DOI valid (prefix subdivizat), iar `11.1000/abc` nu este DOI.
3. În „vezi doi:10.1000/xyz123.” extragerea întoarce `10.1000/xyz123`; valoarea izolată `10.1000/xyz123.` din câmpul doi rămâne cu punct.
4. `https://doi.org/10.1000/ab%23cd` dă canonic `10.1000/ab#cd`.
5. ISBN `978-92-95055-12-4` este valid (suma ponderată 126 + 4 = 130, divizibilă cu 10); `978-92-95055-12-5` are sumă greșită; `9780306406157` este valid.
6. ISSN `0317-8471` este valid (suma 120, control 1); `2434-561X` este valid (suma 122, control 10 → X); `0317-8472` are sumă greșită.
7. ORCID `0000-0002-1825-0097` este valid (control 7, exemplul sursei); `0000-0002-1694-233x` se normalizează la `…233X` și este valid; `0000-0002-1825-0098` are sumă greșită; afișarea este `https://orcid.org/0000-0002-1825-0097`.
8. [necesită sursa ISBN-10, necitită] `0-306-40615-2` este ISBN-10 valid și se convertește în `9780306406157`.

## Necunoscute
- Metoda ISBN-10 și conversia (nu au apărut în pagina citită); testul 8 e condiționat.
- Gruparea cu cratime a ISBN și dacă ISBN-uri 979 sunt acceptate de toate sistemele din aval.
- Gazda `dx.doi.org` și alte forme de proxy ale DOI.
- Reprezentarea standard a ORCID asociat unui autor în formatele bibliografice.

## Verificare (verificare.md)

Spec: 02-persistent-identifiers   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
