# Funcție: Detectarea intrărilor bibliografice duplicate după identificatori

Sursă publică:
- *DOI Handbook*, §2.4 „Case sensitivity” și §2.2 (sintaxa numelui DOI) — https://www.doi.org/doi-handbook/ ; pagina citită: https://albany.doi.org/doi_handbook/2_Numbering.html
- *ISBN Users' Manual*, ed. a 7-a (structura ISBN-13, cifra de control, hyphenation fără semnificație) — https://www.nb.no/content/uploads/2018/04/ISBN-International-Users-Manual-7th-edition_absolutely_final.pdf
- Pentru legătura între tip de lucrare și ISBN: *CSL 1.0.2 Specification*, Appendix III (`book`, `chapter`) — https://raw.githubusercontent.com/citation-style-language/documentation/master/specification.rst
- Crossref, *REST API format* (câmpul `relation` al lucrărilor) — https://raw.githubusercontent.com/Crossref/rest-api-doc/master/api_format.md (licență CC BY 4.0)

Stare: candidat · Dimensiune: S · Dependențe: 01, 02 · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Bibliografia unei teze se adună din mai multe fișiere (`.bib`, RIS, CSL JSON, plicuri). Aceeași lucrare apare cu chei diferite, iar citările cu o cheie nu se potrivesc cu cealaltă, referința se dublează în listă, iar lucrarea citată apare de două ori în rapoarte „neciticate”.

## Intrări / ieșiri
- Intrare: lista completă de intrări bibliografice încărcate (din toate fișierele).
- Ieșire: grupuri de intrări duplicate (cu motivul: același DOI sau același ISBN-13), conflicte (aceeași identificare, metadate care se contrazic) și, la cerere, lista unificată în care fiecare grup devine o singură intrare ce păstrează toate cheile ca alias.
- Doar identificatorii decid; potrivirea aproximativă pe titlu nu face parte din această funcție [dedus].

## Comportament observabil
- Starea actuală: intrările din fișiere diferite se adaugă una după alta; potrivirea unui document se face cu prima cheie sau câmp `file` găsit; nu există detectare de duplicate (cod: modulul bib, încărcarea) [observat].
- Două intrări sunt duplicate dacă au același DOI canonic (spec 02): numele DOI nu face distincție între majuscule și minuscule pentru caractere ASCII [observat], deci `10.1000/ABC` și `10.1000/abc` coincid [dedus din observație].
- Două intrări sunt duplicate după ISBN dacă au același ISBN-13 canonic (un ISBN-10 valid se convertește mai întâi) și ambele sunt cărți întregi (`book`, `proceedings`, `manual`, `booklet`); ISBN-ul unei cărți apare și la capitolele ei, deci nu decide pentru `chapter` / `inbook` / `incollection` [dedus; Appendix III spune că `chapter` este o parte citată separat de carte [observat]].
- Cratimele și spațiile dintr-un ISBN nu contează [observat].
- Cheia unificată = cheia primei intrări în ordinea încărcării; celelalte devin alias și rezolvă la aceeași intrare, inclusiv în citările `[@alias]` [dedus].
- Câmpurile intrării unificate: pentru fiecare câmp, valoarea intrării cu cele mai multe câmpuri completate; dacă două valori nevide diferă, câmpul intră în raportul de conflicte și rămâne valoarea primei intrări [dedus].
- Conflict = același DOI/ISBN dar an diferit, sau titlu cu mai puțin de jumătate din jetoanele comune (jetoane ca la căutare); se raportează, nu se unifică automat [dedus; pragul e [necunoscut] până la măsurare pe date reale].
- O intrare fără niciun identificator nu participă; raportul numără câte au rămas neverificabile [dedus].

## Formate / protocoale
- Raport: pentru fiecare grup, cheile, fișierele de origine, motivul și, la conflict, câmpurile care diferă. [dedus]
- Aliasurile se salvează în raport, nu se scriu înapoi în fișierele `.bib` ale utilizatorului [dedus].

## Cazuri limită și erori
- DOI nevalid ca sintaxă (spec 02): nu participă la grupare, se raportează ca nevalid [dedus].
- ISBN cu sumă greșită: nu participă; se raportează [dedus].
- Trei intrări, două cu același DOI și a treia cu un ISBN comun cu una dintre ele: grup unic, tranzitiv [dedus].
- Aceeași cheie în două fișiere pentru lucrări diferite: nu este duplicat; este coliziune de chei și se raportează separat [dedus].

## Teste de acceptanță
1. Intrări cu DOI `10.1000/ABC` și `10.1000/abc` → un grup, motiv „același DOI”.
2. Cărți cu ISBN `978-92-95055-12-4` și `9789295505124` → un grup, motiv „același ISBN”.
3. Capitol (`chapter`) și carte cu același ISBN → fără grup.
4. Același DOI, ani 2019 și 2017 → raportat ca conflict, nu unificat.
5. Unificare: după unificare, citarea `[@alias]` se rezolvă la intrarea unificată, iar lista de referințe conține lucrarea o singură dată.
6. Intrare fără identificatori + intrare cu DOI → fără grup, a doua rămâne; numărătoarea „neverificabile” = 1.
7. ISBN cu sumă greșită (`978-92-95055-12-5`) → nu intră în niciun grup și este raportat.

## Necunoscute
- Pragul de conflict pe titlu și dacă merită potrivire aproximativă (nu s-a citit nicio sursă publică pentru potrivire aproximativă; funcția nu o include).
- Cum ar trebui tratate versiunile diferite ale aceleiași lucrări sub DOI diferite (preprint și articol final); Crossref are relații între lucrări [observat că există câmpul de relații în formatul răspunsurilor], dar utilizarea lor nu e specificată aici.

## Verificare (verificare.md)

Spec: 07-duplicate-by-identifier   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
