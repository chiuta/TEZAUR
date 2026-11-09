# Funcție: Analiza numelor de persoane din câmpurile de autori

Sursă publică:
- *BibTeXing* (Oren Patashnik), secțiunea despre formatul numelor — https://mirrors.ctan.org/biblio/bibtex/base/btxdoc.tex (citit prin oglinda https://ipv4.mirrors.cqupt.edu.cn/ctan/biblio/bibtex/base/btxdoc.tex)
- Documentația „authors” a unui analizor independent de nume BibTeX, doar pentru exemplele cu acolade — https://ftp.gwdg.de/pub/ctan/biblio/beastie/doc/authors.md (sursă secundară; citită parțial, rezumat)
- *CSL 1.0.2 Specification*, secțiunea „Name Particles” și variabilele de nume (părțile `family`, `given`, `dropping-particle`, `non-dropping-particle`, `suffix`, `literal`) — https://raw.githubusercontent.com/citation-style-language/documentation/master/specification.rst ; schema https://raw.githubusercontent.com/citation-style-language/schema/master/schemas/input/csl-data.json

Stare: candidat · Dimensiune: M · Dependențe: niciuna (folosită de 03, 05, 06) · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Citările de tip „Autor & Autor (an)”, lista de referințe, exportul CSL JSON și deduplicarea au nevoie de nume separate corect în prenume, particulă, nume de familie și sufix. Heuristica actuală (ultimul cuvânt, plus o listă mică de cuvinte de instituție) greșește la nume compuse, particule și sufixe.

## Intrări / ieșiri
- Intrare: valoarea unui câmp de autori sau editori (șir de nume separate prin cuvântul `and`).
- Ieșire: listă ordonată de persoane; fiecare are Prenume (First), Particulă (von), Nume (Last), Sufix (Jr), toate liste de jetoane posibil goale, sau un nume literal (instituție); plus un indicator „și alții” când lista se încheie cu `and others`.
- Exemplu: `Doe, John and others` → o persoană (Last Doe, First John) și indicatorul „și alții”.

## Comportament observabil
- Starea actuală (cod: modulul bib): lista se desparte la „ and ” scris cu litere mici; `and others` produce un „autor” numit others; numele cu virgulă are numele înainte de virgulă; fără virgulă, ultimul cuvânt este numele, în afară de cazul în care apare unul din douăsprezece cuvinte de instituție (of, for, the, council, university, institute, commission, union, association, ministry, organization, organisation), caz în care tot șirul e nume; particulele, sufixele și prenumele separate nu există [observat].
- Mai multe nume într-un câmp se despart prin cuvântul `and`, înconjurat de spații și neînchis în acolade [observat]. Lista poate fi încheiată cu `and others`, pe care stilurile îl redau ca „et al.” [observat].
- Fiecare nume are patru părți, fiecare o listă (posibil goală) de jetoane: First, von, Last, Jr; partea Last este nevidă dacă oricare parte este nevidă [observat].
- Trei forme: `First von Last`, `von Last, First`, `von Last, Jr, First` [observat]. Prima poate fi folosită aproape întotdeauna; nu se folosește când există Jr sau când Last are mai multe jetoane fără von [observat].
- Un jeton este von dacă prima literă aflată la nivel de acolade 0 este literă mică [observat]. Un caracter special între acolade (de exemplu o literă accentuată scrisă ca secvență LaTeX în acolade) este tratat ca la nivel 0, deci contează litera lui [observat]. Prin prefixarea unui jeton cu o acoladă goală se poate forța încadrarea ca von sau non-von [observat; mecanism exact — necunoscut].
- Exemplu din manual: `Charles Louis Xavier Joseph de la Vall{\'e}e Poussin` are patru jetoane în First, două în von și două în Last [observat].
- Forma greșită `Per Brinch Hansen` face din „Brinch” un jeton First; forma corectă este `Brinch Hansen, Per` [observat].
- Acoladele în jurul unui șir îl fac un singur nume: `{Barnes and Noble, Inc}` este un nume având numai Last; `Andreas {Thurn and Taxis}` este un nume cu First „Andreas” și Last „Thurn and Taxis”; un nume dintr-un singur jeton este Last [observat din sursa secundară].
- Cum se decide care jetoane sunt Last și care von când toate sunt cu literă mică, sau cum se tratează jetoanele care încep cu cifră ori simbol, nu rezultă din textele citite [necunoscut].
- Către CSL JSON: First → `given`; Last → `family`; Jr → `suffix`; nume literal → `literal`; von → `non-dropping-particle` (BibTeX nu distinge particulele care cad de cele care rămân) [dedus]. CSL descrie particulele care rămân și cele care cad la afișarea doar a numelui de familie: în exemplele sale, „van” din „Vincent van Gogh” rămâne („Van Gogh”), iar „von” din „Alexander von Humboldt” cade („Humboldt”) [observat]. Alegerea `non-dropping-particle` păstrează particula în citări, ceea ce corespunde numelor ca „van Gogh” și poate fi nepotrivită pentru „von Humboldt” [dedus].
- Nume afișat în citare (ex. „Smith & Doe (2019)”) = von + Last, fără prenume; pentru nume literale = șirul întreg [dedus].
- Indicatorul „și alții” produce în citare „… et al.” [observat pentru stiluri; aplicare dedusă].

## Formate / protocoale
- Forma internă a numelui: patru liste de jetoane, fiecare jeton păstrând acoladele originale; curățarea LaTeX se face după analiză, pe fiecare jeton [dedus].
- Rata de eroare tolerată: nicio excepție; orice șir produce o persoană (cel puțin Last) [dedus].

## Cazuri limită și erori
- Șir gol sau doar spații: nicio persoană. [dedus]
- Mai mult de două virgule într-un nume: manualul nu definește; se tratează ca a treia formă și restul virgulelor rămân în First [dedus; necunoscut].
- `and` cu majuscule (`AND`) sau scris lipit: nu desparte nimic dacă nu e cu litere mici înconjurat de spații [dedus; manualul nu spune dacă potrivirea ignoră majusculele — necunoscut].
- `and others` urmat de alt nume: „others” nu e ultimul; se tratează ca nume literal [dedus].
- Nume cu un singur jeton (instituție fără acolade): Last [observat din sursa secundară].

## Teste de acceptanță
1. `Charles Louis Xavier Joseph de la Vall{\'e}e Poussin` → First: 4 jetoane (Charles Louis Xavier Joseph), von: 2 (de la), Last: 2 (Vall{\'e}e Poussin).
2. `Brinch Hansen, Per` → Last „Brinch Hansen”, First „Per”; `Per Brinch Hansen` → First „Per Brinch”, Last „Hansen”.
3. `von Beethoven, Ludwig` → von „von”, Last „Beethoven”, First „Ludwig”.
4. `Ford, Jr., Henry` → Last „Ford”, Jr „Jr.”, First „Henry”.
5. `{Barnes and Noble, Inc}` → o singură persoană cu doar Last; `Alpha, A. and Beta, B. and Gamma, C.` → trei persoane.
6. `Andreas {Thurn and Taxis}` → First „Andreas”, Last „Thurn and Taxis”.
7. `Doe, John and others` → o persoană și indicatorul „și alții”; nu apare niciun autor numit others.
8. Export CSL: `von Beethoven, Ludwig` → `family` „Beethoven”, `non-dropping-particle` „von”, `given` „Ludwig”.

## Necunoscute
- Algoritmul exact pentru cazurile ambigue (toate jetoanele cu literă mică, jetoane care încep cu cifră).
- Sensibilitatea la majuscule a cuvântului `and`.
- Alegerea între particulă care cade și particulă care rămâne la export CSL pentru nume reale (depinde de limbă și de uzul persoanei).
- Tratarea numelor est-asiatice (ordine invers) și a numelor românești cu prenume compuse scrise fără virgulă.

## Verificare (verificare.md)

Spec: 04-bibtex-person-names   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
