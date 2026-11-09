# Funcție: Căutare de expresie exactă și de apropiere între cuvinte

Sursă publică:
- Manning, Raghavan, Schütze, *Introduction to Information Retrieval* (Cambridge University Press, 2008), cap. 2.4 „Positional postings and phrase queries” — https://nlp.stanford.edu/IR-book/html/htmledition/positional-indexes-1.html (versiunea online; citit rezumat în sesiune, inclusiv exemplul „to be or not to be” și interogarea de apropiere „employment /3 place”)
- Pentru jetoane și poziții: spec 09 din acest set (bazat pe UAX #29 și UAX #15)

Stare: candidat · Dimensiune: L · Dependențe: 09 · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Cercetătorul caută o formulare exactă („sarcina probei”, „mutual recognition”) sau două cuvinte aflate aproape, și vrea doar pasajele în care apar astfel. Căutarea actuală tratează interogarea ca o mulțime de cuvinte, fără ordine și fără apropiere, deci „burden of proof” găsește și „proof of the burden”.

## Intrări / ieșiri
- Intrare: interogare cu text între ghilimele duble (drepte sau tipografice) și, opțional, un sufix de apropiere; restul cuvintelor sunt termeni obișnuiți.
- Ieșire: aceleași rezultate ca la căutarea actuală (pasaje cu fișier, pagină, scor), dar pasajele trebuie să conțină expresia, respectiv perechea de cuvinte la distanța cerută.
- Exemplu: `"burden of proof" authority` → pasaje care conțin expresia „burden of proof” și, ca termen obișnuit, „authority” (care contribuie la scor).

## Comportament observabil
- Starea actuală: interogarea se simbolizează și se punctează fiecare termen distinct cu formula BM25 (k1 = 1,5; b = 0,75), fără informație de poziție; rezultatele se combină cu căutarea densă prin fuziune de ranguri, iar un avertisment spune ce procent din termeni apare în cel mai bun pasaj (cod: modulul de index) [observat].
- O interogare de expresie se evaluează cu informație de poziție: termenii trebuie să apară în aceeași ordine, pe poziții consecutive; coincidența simplă nu ajunge [observat în sursă].
- Procedura clasică pornește de la termenul cel mai rar, restrânge documentele candidate și verifică pozițiile; indexul pozițional păstrează pentru fiecare termen documentele și pozițiile [observat]. În exemplul din carte, „to” la pozițiile 429 și 433 și „be” la 430 și 434 în documentul 4 dau o potrivire pentru „to be” [observat].
- Interogarea de apropiere „employment /3 place” înseamnă că cele două cuvinte apar la cel mult 3 cuvinte unul de altul, în orice ordine [observat]. Indexul pozițional rezolvă astfel de interogări; un index de perechi de cuvinte (biword) nu [observat].
- Sintaxa acestei funcții (decizie): text între ghilimele = expresie; după ghilimelele de închidere, `~` și un număr întreg k ≥ 1 cere apropiere, definită doar pentru exact două cuvinte (sursa definește apropierea pentru perechi); expresie cu un singur cuvânt = termen obișnuit [dedus].
- Metoda de evaluare: se iau pasajele candidate care conțin toți termenii expresiei (liste de apariții deja existente), apoi textul fiecărui candidat se segmentează cu aceeași funcție ca la indexare (spec 09) și se verifică pozițiile; nu se modifică indexul și memoria rămâne ca azi [dedus; rezultatul e echivalent cu cel al unui index pozițional, dar nu s-a măsurat costul — necunoscut].
- Pozițiile numără toate cuvintele segmentate, inclusiv cele pe care indexul le elimină (sub două caractere), deci „state claim” nu potrivește „state a claim” [dedus].
- Scorul: BM25 pe termenii expresiei și pe termenii obișnuiți, doar pentru pasajele care satisfac expresia; pasajele care nu o satisfac sunt excluse, nu penalizate [dedus].
- Ghilimele tipografice („ ”, “ ”) și drepte delimitează la fel [dedus].
- Expresiile nu traversează granițele de pasaj: textul se taie pe pagini și în ferestre de aproximativ 200 de cuvinte cu suprapunere de 40 [observat în documentația proiectului]; o expresie mai scurtă decât suprapunerea apare întreagă în cel puțin un pasaj în interiorul unei pagini; la trecerea de pagină nu se găsește [dedus].
- Dacă două pasaje suprapuse conțin amândouă expresia, ambele rămân în rezultate; deduplicarea e [necunoscut].
- Filtrele (document, `--only`), plafonul pe document și fuziunea cu rezultatele dense rămân neschimbate: filtrul de expresie se aplică înainte de clasare [dedus].
- Avertismentul de potrivire slabă folosește termenii expresiei și cei obișnuiți [dedus].

## Formate / protocoale
- Sufixul de apropiere: `"cuvânt1 cuvânt2"~3`; spațiile din expresie separă cuvintele segmentate; punctuația din interiorul expresiei urmează segmentarea (spec 09), deci `"3.5"` este un cuvânt [dedus].
- Interfețe: același text de interogare în linia de comandă, interfața web, serverul local și serverul pentru unelte externe [dedus; azi toate pornesc de la aceeași căutare — cod].

## Cazuri limită și erori
- Ghilimele neînchise: restul interogării se tratează ca termeni obișnuiți, cu un avertisment [dedus].
- Expresie goală `""`: ignorată. [dedus]
- Expresie cu un cuvânt care are un singur caracter (eliminat de index): expresia nu poate fi verificată; avertisment, ignorată [dedus].
- Apropiere cu k = 0 sau negativ: eroare de sintaxă, interogarea nu rulează [dedus].
- Apropiere pe mai mult de două cuvinte: eroare de sintaxă [dedus].
- Interogare formată doar din expresii fără rezultate: lista goală, ca la căutarea actuală. [dedus]

## Teste de acceptanță
1. Corpus: A = „The burden of proof lies with the authority”, B = „The proof of the burden is heavy”. Interogarea `"burden of proof"` returnează doar A; interogarea fără ghilimele le returnează pe amândouă.
2. Cu A = „state a claim”, interogarea `"state claim"` nu o returnează, iar `"state a claim"` o returnează.
3. Interogarea `"employment place"~3` potrivește „place of employment” (distanță 2, ordine inversă) și nu potrivește „employment … place” cu cinci cuvinte între ele.
4. `"Burden of Proof"` potrivește „burden of proof” (aceeași normalizare ca la indexare, spec 09).
5. `"burden of proof` (ghilimele neînchise) → același rezultat ca fără ghilimele, plus avertisment.
6. `"proof"` (un singur cuvânt) → rezultate identice cu `proof`.
7. Combinație: `"burden of proof" authority` returnează doar pasajele care conțin expresia; între ele, cele care conțin și „authority” au scor mai mare decât celelalte.
8. Expresie care ar trece peste granița dintre pagina 3 și pagina 4 nu se găsește; aceeași expresie întreagă pe pagina 4 se găsește.

## Necunoscute
- Costul pe biblioteci mari (verificarea pe candidați vs. index pozițional cu poziții stocate).
- Sintaxa dorită de utilizatori (alte forme de apropiere, sufixul `~` vs. `/k`).
- Tratamentul expresiilor cu cuvinte-umplutură în limbi cu flexiune bogată (prefixul de 7 caractere poate face potriviri mai largi decât cele așteptate).
- Deduplicarea pasajelor suprapuse.

## Verificare (verificare.md)

Spec: 10-phrase-query   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
