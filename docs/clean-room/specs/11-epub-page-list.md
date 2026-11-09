# Funcție: Pagini tipărite pentru cărți EPUB (lista de pagini și marcajele de pagină)

Sursă publică:
- *EPUB 3.3*, Recomandare W3C, 2023-05-25 — https://www.w3.org/TR/epub-33/ (versiunea datată: https://www.w3.org/TR/2023/REC-epub-33-20230525/ ); textul normativ a fost citit din sursa publică https://raw.githubusercontent.com/w3c/epub-specs/main/epub33/core/index.html (licență „W3C Software and Document License”), în special secțiunea „The page-list nav element” (sec-nav-pagelist) și exemplele cu `epub:type="pagebreak"` și `role="doc-pagebreak"`
- Vocabularul `pagebreak` este definit în *EPUB 3 Structural Semantics Vocabulary* (epub-ssv), referit de specificația de mai sus și necitit în sesiune
- Identificarea sursei paginării (ediția tipărită la care se referă numerele) este tratată în *EPUB Accessibility 1.1*, necitit în sesiune

Stare: candidat · Dimensiune: L · Dependențe: 08 (localizatori), 09 (segmentare) · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Documentația proiectului spune că EPUB și DOCX nu au numere de pagină, deci citările către cărți electronice pierd pagina. Multe cărți EPUB poartă totuși lista paginilor edițiilor tipărite. Folosirea ei permite citări „p. 14” verificabile pentru cărți în EPUB.

## Intrări / ieșiri
- Intrare: un fișier EPUB.
- Ieșire: textul cărții în ordinea de lectură, împărțit în pagini; fiecare pagină are o etichetă (șir: „14”, „xii”) și un număr de ordine; textul de dinaintea primului marcaj nu are pagină.
- Un indicator al documentului arată dacă paginile provin dintr-o listă de pagini, din marcaje în text sau lipsesc.

## Comportament observabil
- Starea actuală: textul tuturor documentelor de conținut din ordinea spinării (itemref) se lipește într-un singur șir, documentul de navigare (proprietatea `nav` din manifest) este exclus, iar întregul text primește pagina 0 („fără pagini”); citările către EPUB nu pot fi verificate pe pagină (cod: modulul de ingestie; README) [observat].
- Lista de pagini este un element `nav` al documentului de navigare cu `epub:type="page-list"`; el oferă navigare la limitele statice de pagină, care pot corespunde unei surse tipărite sau pot fi definite doar pentru publicație [observat].
- Elementul este opțional și apare cel mult o dată; ar trebui să conțină un singur `ol`, fără liste imbricate; fiecare element e o legătură a cărei țintă este locul unde începe pagina [observat]. Destinațiile pot fi identificate în documentele de conținut prin termenul `pagebreak` [observat].
- Elementul `nav` poate avea atributul `hidden`, ca lista să nu apară în fluxul de citire [observat]. Etichetele paginilor sunt textul legăturii (în exemplul din specificație, „I” pentru o pagină romană din ediția a treia tipărită) [observat], deci o etichetă nu este în mod necesar un număr întreg.
- În documentele de conținut un marcaj de pagină poate fi un element gol cu `id` și `role="doc-pagebreak"` sau `epub:type="pagebreak"`; eticheta poate fi în `aria-label` (exemplul: eticheta „14”) [observat]. Alte locuri pentru etichetă (`title`, text) nu rezultă din textele citite [necunoscut].
- Căutarea marcajelor: (1) lista de pagini din documentul de navigare, dacă există; (2) altfel, elementele cu `epub:type="pagebreak"` sau `role="doc-pagebreak"` din documentele spinării, în ordinea lor [dedus].
- Rezolvarea unei legături: referința `fișier#id` se caută în documentul respectiv ca poziție în text; legătura fără fragment înseamnă începutul fișierului [dedus]. Textul se segmentează la fiecare poziție de marcaj, în ordinea lecturii: de la un marcaj la următorul formează o pagină cu eticheta marcajului precedent [dedus].
- Lista de pagini nu e garantat în ordinea de lectură; ordinea pozițiilor din text este cea care contează, iar etichetele duplicate rămân pagini separate cu număr de ordine distinct [dedus; garanția de ordine este necunoscută].
- Un pasaj indexat nu trece peste granița de pagină (ca la PDF azi) [observat pentru PDF în cod; extins dedus].
- Pagina documentului: numărul „de pagini” = cea mai mare etichetă numerică (arabă) întâlnită; etichetele romane nu intră în această valoare [dedus]; etichetele romane se pot cita (spec 08) și se compară ca atare [dedus].
- Neutralitate: etichetele provin de la editorul cărții și pot desemna ediția tipărită, nu fișierul EPUB; raportul de verificare menționează că numărul paginii este cel al sursei paginării declarate de carte, nu o poziție în fișier [dedus].
- Identificarea ediției tipărite pe care o numără paginile (editor, an, ISBN) este în metadatele cărții; cum se citesc acestea nu rezultă din sursele citite [necunoscut]; în lipsă, raportul spune doar că ediția nu este cunoscută.
- Dacă nu există nici listă de pagini, nici marcaje, comportamentul rămâne cel actual: pagina 0. [dedus]
- Marcajele goale (fără etichetă) se ignoră, dar poziția lor nu începe o pagină nouă [dedus].

## Formate / protocoale
- Documentul de navigare: XHTML; elementul `nav` cu `epub:type` conținând `page-list`; intrările `li` / `a` cu `href` relativ la documentul de navigare [observat]; rezolvarea căilor ține cont de directorul acestui document (ca în cod azi pentru spinare) [dedus].
- Etichetă: șir Unicode, fără normalizare; cifrele romane se recunosc ca șir de caractere i, v, x, l, c, d, m (majuscule sau minuscule) [dedus].
- Legăturile percent-codificate se decodifică înainte de căutare [dedus].

## Cazuri limită și erori
- Legătură către un fișier care nu este în spinare: ignorată, cu avertisment. [dedus]
- Fragment inexistent: pagina începe la începutul fișierului; avertisment [dedus].
- EPUB cu mai multe liste de pagini (încalcă „cel mult o dată”): se folosește prima [observat pentru regula; comportament dedus].
- EPUB 2 (fără document de navigare în sensul EPUB 3): această funcție nu se aplică; rămâne comportamentul actual [dedus].
- Pagini cu etichete nenumerice care nu sunt romane (ex. „A-3”): acceptate ca etichete, necomparabile numeric [dedus].

## Teste de acceptanță
1. EPUB cu un document `c1.xhtml` care conține „Alpha. <span id="p1" epub:type="pagebreak"/> Beta. <span id="p2" epub:type="pagebreak"/> Gamma.” și un `nav` page-list cu legăturile `c1.xhtml#p1` (etichetă „1”) și `c1.xhtml#p2` („2”) → „Alpha” fără pagină, „Beta” pagina 1, „Gamma” pagina 2.
2. Aceeași carte fără `nav` page-list, dar cu `role="doc-pagebreak"` și `aria-label="14"` pe marcaj → „Beta” pe pagina 14.
3. Etichete „xii” și „1” → pagina romană „xii” poate fi citată (`[@carte, p. xii]`), numărul de pagini al documentului este 1 (singura etichetă arabă).
4. Carte fără nicio listă și fără marcaje → pagina 0, ca azi; citarea `[@carte, p. 3]` rămâne neverificabilă pe pagină.
5. Pasaj care ar acoperi două pagini → împărțit; fiecare parte are pagina ei.
6. Legătură din lista de pagini către fișier absent din spinare → ignorată; celelalte pagini rămân corecte.
7. Eticheta 14 apare de două ori (două volume într-un fișier) → două pagini separate; citarea `p. 14` se verifică pe oricare dintre ele.

## Necunoscute
- Cum se identifică ediția tipărită (secțiunea de accesibilitate nu a fost citită).
- Locurile permise pentru etichetă în marcajele din conținut, în afară de `aria-label`.
- Garanția de ordine a listei de pagini.
- Comportamentul pe cărți reale (nu s-a încercat nicio carte; testele sunt sintetice).
- Cum se leagă acest comportament de DOCX, unde numerotarea de pagină depinde de randare [necunoscut, neacoperit aici].

## Verificare (verificare.md)

Spec: 11-epub-page-list   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [x] Licența/EULA țintei verificată și notată în jurnal (licențele surselor citite sunt notate în JURNAL.md (fișierele citite: verificate în depozit))
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
