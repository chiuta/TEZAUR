# Funcție: Accesibilitate a interfeței web (cerințe verificabile WCAG 2.2, nivel A și AA)

Sursă publică:
- *Web Content Accessibility Guidelines (WCAG) 2.2*, Recomandare W3C — https://www.w3.org/TR/WCAG22/ ; criteriile de succes au fost citite din sursa publică https://raw.githubusercontent.com/w3c/wcag/main/guidelines/sc/20/ , …/sc/21/ , …/sc/22/ (fișierele `non-text-content`, `info-and-relationships`, `use-of-color`, `contrast-minimum`, `resize-text`, `keyboard`, `focus-visible`, `language-of-page`, `labels-or-instructions`, `name-role-value`, `reflow`, `non-text-contrast`, `status-messages`, `target-size-minimum`) și definițiile „relative luminance” și „contrast ratio” din …/guidelines/terms/20/ (licență „W3C Document License”)
- Interfața vizată: fișierul `tezaur.html` al depozitului (observat prin citire și prin calcul, nu prin rulare într-un browser sau cu un cititor de ecran)

Stare: candidat · Dimensiune: M · Dependențe: niciuna · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Pagina de lucru (întrebare, rezultat cu citări, panou de audit) este folosită de studenți și cercetători, între care persoane care navighează cu tastatura sau cu cititor de ecran. Funcția definește cerințele de accesibilitate pe care interfața trebuie să le îndeplinească și lista de verificări cu rezultate măsurabile.

## Intrări / ieșiri
- Intrare: interfața web (fișierul unic și variantele care îl reutilizează).
- Ieșire: un raport cu o linie pe criteriu (trecut / eșuat / neverificat), cu elementul vizat și motivul.
- Fiecare cerință de mai jos poate fi verificată pe baza criteriului WCAG citat.

## Comportament observabil

Starea actuală a interfeței (cod: `tezaur.html`):
- Limba paginii este declarată (`lang="ro"`); dacă selectorul limbii interfeței actualizează și atributul `lang` este [necunoscut] [observat pentru atribut].
- Există repere structurale: un titlu de nivel 1, un `aside`, un `main`, titluri de nivel 2; atributele `role` nu apar nicăieri; singurul atribut ARIA folosit este `aria-label` (9 apariții); `aria-live` nu apare [observat].
- Din 18 elemente `label` (și 18 `input`, 6 `select`, 3 `textarea` în fișier), unul are atributul `for`; altele învelesc controlul (casetele de bifare), iar celelalte stau lângă control fără legătură programatică (de exemplu etichetele câmpurilor pentru note, tip de text, document, limba răspunsului, numele nodului); câmpul principal de întrebare are doar text-indiciu (`placeholder`) [observat].
- Filele de mod sunt butoane create dinamic; starea „activ” este transmisă doar printr-o clasă de stil [observat].
- Marcajele de citare din rezultat sunt elemente `a` fără `href`, cu manipulator de clic [observat]; butonul de ștergere al unui document are ca text caracterul „×” și un `title` [observat]; butonul de dictare are doar `title` și un emoji [observat].
- Zonele în care apar stări, avertismente și rezultate (stare bibliotecă, avertismente, rezultat) se actualizează din cod fără rol sau proprietate care să le facă anunțabile [observat].
- Stilul de focus nu este definit în pagină și nicio regulă nu elimină conturul implicit [observat prin lipsa regulilor `outline` și `:focus`]; aspectul real depinde de browser [necunoscut].
- Culorile sunt variabile CSS cu variantă pentru tema întunecată [observat]. Rapoartele de contrast calculate cu formula din definiții (luminanța relativă a sRGB, raport = (L1 + 0,05) / (L2 + 0,05)) [observat pentru formulă; valorile sunt calculate de autorul specificației]:
  - text principal pe fundal: 14,63:1 (deschis), 14,66:1 (întunecat);
  - text atenuat pe fundal: 5,11:1 (deschis), 6,00:1 (întunecat); pe panou: 5,52:1 și 5,58:1;
  - text pe butonul principal (culoarea panoului pe culoarea de accent): 6,67:1 și 7,74:1;
  - culori de stare pe fundal: eroare 6,61:1 / 7,98:1; reușită 5,82:1 / 9,66:1; avertisment 4,61:1 / 10,35:1;
  - culoarea liniilor / marginilor câmpurilor pe fundal: 1,32:1 (deschis) și 1,46:1 (întunecat).

Cerințe [dedus: aplicarea criteriilor la elementele de mai sus; textul criteriilor este [observat]]:
1. Conținut ne-text și nume (1.1.1, 4.1.2, 3.3.2): fiecare control are un nume accesibil: etichete asociate programatic (prin `for`/`id` sau învelire) sau `aria-label`; butoanele fără text vizibil (ștergere, dictare) au nume explicit. Criteriul 4.1.2 cere ca numele și rolul să fie determinabile programatic și notificările de schimbare disponibile; 3.3.2 cere etichete sau instrucțiuni pentru câmpurile de intrare.
2. Structură și relații (1.3.1): informația transmisă prin prezentare (titluri, grupări de câmpuri, liste de documente, tabel de carduri) poate fi determinată programatic sau există în text. Filele sunt expuse ca grup cu starea selectată determinabilă (criteriul 4.1.2).
3. Tastatură (2.1.1): toată funcționalitatea se folosește de la tastatură; marcajele de citare din rezultat sunt controale focalizabile și activabile de la tastatură, cu aceeași acțiune ca la clic.
4. Focus vizibil (2.4.7): orice control operabil de la tastatură are un mod în care indicatorul de focus este vizibil; indicatorul menținut este cel implicit sau unul definit cu contrast de cel puțin 3:1 față de culorile vecine (1.4.11) [dedus pentru valoarea 3:1].
5. Mesaje de stare (4.1.3, AA): mesajele de stare (încărcare terminată, număr de pasaje găsite, avertismente, progres, erori) pot fi determinate programatic prin rol sau proprietăți, ca tehnologiile asistive să le prezinte fără mutarea focusului. Rezultatul care se completează progresiv nu se anunță cuvânt cu cuvânt; se anunță începutul și sfârșitul [dedus].
6. Contrast (1.4.3, AA): text de cel puțin 4,5:1, text mare de cel puțin 3:1, în ambele teme; valorile actuale citate mai sus trec, inclusiv avertismentul din tema deschisă la 4,61:1 (marja este mică). Culorile noi se adaugă numai după același calcul.
7. Contrast pentru non-text (1.4.11, AA): componentele de interfață și starea lor (marginea câmpurilor, a butoanelor secundare) au cel puțin 3:1 față de culorile vecine, cu excepția cazului în care aspectul este stabilit de browser; marginile actuale ale câmpurilor (1,32:1 și 1,46:1) sunt sub prag și nu sunt compensate de altă marcare vizibilă a câmpului [dedus: aplicarea excepției este o judecată]. Fișa specifică 1.4.11 prin criteriul `non-text-contrast` citit [observat].
8. Culoare (1.4.1): culoarea nu este singurul mijloc de a transmite informația; citarea inexistentă (stilată cu culoare și linie de tăiere) respectă cerința; starea unei file active (doar culoare de fundal și margine) trebuie dublată de semnal programatic și, vizual, de un alt indiciu [dedus].
9. Redimensionare și refluare (1.4.4, 1.4.10): textul se mărește până la 200% fără pierderi, iar conținutul este utilizabil la lățimea echivalentă a 320 de pixeli CSS fără derulare în două dimensiuni, cu excepția părților care cer aranjare bidimensională (tabelele cu mai multe coloane) [observat pentru text criteriilor]. Interfața conține o regulă pentru ecrane înguste (sub 820 px) [observat]; comportamentul real la 320 px este [necunoscut].
10. Dimensiunea țintelor (2.5.8, AA nou în 2.2): țintele pentru indicatori de tip pointer sunt de cel puțin 24×24 pixeli CSS, cu excepțiile din criteriu (spațiere, echivalent, în linie, control de agent utilizator) [observat pentru criteriu]. Butonul de ștergere (fără umplutură verticală, doar „×”) și marcajele de citare în linie sunt cele de verificat [dedus].
11. Limba (3.1.1): atributul `lang` al documentului corespunde limbii interfeței alese și se actualizează la schimbarea ei [dedus]; limba rezultatului generat (cerută de utilizator) nu e marcată pe fragmentul respectiv; criteriul pentru părți de pagină (3.1.2) nu a fost citit [necunoscut].
12. Conținut generat: tabelul de carduri și lista de pasaje au anteturi de coloană programatice (1.3.1) [dedus].

## Formate / protocoale
- Raportul de verificare: tabel cu criteriul (număr și nume), nivelul (A / AA), elementul (selector), rezultatul și măsurătoarea (de exemplu raportul de contrast cu două zecimale). [dedus]
- Calculul contrastului: transformare sRGB → luminanță relativă cu pragul 0,04045 (nu 0,03928, valoarea veche, cu efect practic nul) [observat din nota definiției].

## Cazuri limită și erori
- Culoare cu transparență: contrastul se calculează față de fundalul efectiv; fără fundal specificat, criteriul de contrast e considerat eșuat, pentru că fundalul utilizatorului nu e cunoscut [observat în definiția raportului de contrast: este eșec când nu e specificat fundalul].
- Text imagine (emoji ca singur conținut al unui buton): are nume prin atribut [dedus].
- Elemente ascunse cu `hidden`: nu intră în verificări de focus și nume. [dedus]
- Componente generate de scripturi: verificate după randare, nu după șablon [dedus].

## Teste de acceptanță
1. Fiecare câmp de introducere (`input`, `select`, `textarea`), inclusiv câmpul de întrebare, are nume accesibil nevid; numărul de câmpuri fără nume este 0.
2. Marcajele de citare din rezultat pot fi atinse cu Tab și activate cu Enter, cu același efect ca la clic.
3. Un mesaj de stare adăugat în regiunea de stare este anunțat de un cititor de ecran fără mutarea focusului (rol sau proprietate prezentă pe regiune înainte de actualizare).
4. Filele de mod expun starea selectată în arborele de accesibilitate (exact una selectată).
5. Raportul de contrast pentru fiecare pereche text–fundal din ambele teme este ≥ 4,5:1; pentru margini ale câmpurilor ≥ 3:1 (azi: 1,32:1 și 1,46:1, deci testul eșuează înainte de modificare).
6. La 320 de pixeli CSS lățime, nicio derulare orizontală a paginii în afara tabelelor.
7. Butonul de ștergere al unui document are ținta ≥ 24×24 px CSS sau o țintă echivalentă.
8. La schimbarea limbii interfeței, `lang` al documentului se schimbă în același sens.

## Necunoscute
- Comportamentul cu cititoare de ecran reale (nu s-a rulat niciunul).
- Aspectul indicatorului de focus în browserele țintă.
- Comportamentul real la 320 px și la 200% mărire.
- Criteriile WCAG 2.2 care nu au fost citite (de exemplu 2.4.11 „Focus Not Obscured”, 3.2.6, 3.3.7, 3.3.8).
- Variantele (extensie, aplicații desktop, PWA) care reutilizează pagina: se presupune moștenirea aceleiași interfețe [dedus].

## Verificare (verificare.md)

Spec: 15-ui-accessibility   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [x] Licența/EULA țintei verificată și notată în jurnal (licențele surselor citite sunt notate în JURNAL.md (fișierele citite: verificate în depozit))
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
