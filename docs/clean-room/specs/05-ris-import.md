# Funcție: Import de bibliografie în format RIS

Sursă publică:
- *RIS (file format)*, descrierea liniilor, a etichetelor și a codurilor de tip — https://en.wikipedia.org/wiki/RIS_(file_format) (sursă secundară; text sub CC BY-SA, doar faptele sunt folosite; citită integral în sesiune)
- Specificația RIS originală (Research Information Systems) nu a fost accesibilă în sesiune; diferențele față de sursa secundară sunt [necunoscut].

Stare: candidat · Dimensiune: M · Dependențe: 01 (tipuri), 02 (identificatori), 04 (nume) · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Bazele de date și cataloagele universitare oferă exporturi RIS; programul scrie RIS, dar nu îl poate citi. Studentul care are doar un fișier RIS nu poate lega documentele indexate de referințe, deci citările apar fără autor și an.

## Intrări / ieșiri
- Intrare: un fișier text `.ris` (una sau mai multe înregistrări).
- Ieșire: aceeași listă de intrări bibliografice pe care o produce citirea BibTeX (cheie, tip, autori, an, titlu, container, doi, url, fișier, notă etc.), plus avertismente.
- Cheie de potrivire cu documentele indexate: câmpul de identificare al înregistrării, altfel una generată (vezi mai jos); legătura cu un fișier se face și prin eticheta de atașament local, ca azi prin câmpul `file` din BibTeX (cod) [dedus].

## Comportament observabil
- Starea actuală: RIS se poate doar scrie (tip JOUR sau GEN, câmpurile ID, AU, TI, T2, PY, DO, ER); nu există cititor (cod: bib, export) [observat].
- Fiecare linie are o etichetă de două caractere, două spații, o cratimă, un spațiu și valoarea (`TY  - JOUR`) [observat].
- Specificația cere sfârșit de linie CRLF; sistemele Unix folosesc de obicei doar LF [observat]. Cititorul acceptă CRLF, LF și CR [dedus].
- `TY` trebuie să fie prima etichetă a unei înregistrări, `ER` ultima; celelalte pot veni în orice ordine și sunt opționale; înregistrările se succed direct, fără linie goală [observat]. Cititorul tolerează linii goale între înregistrări [dedus].
- Majoritatea etichetelor apar o singură dată; autorii, cuvintele-cheie și URL-urile se pot repeta [observat].
- Autor: o etichetă pe persoană, în forma „nume, prenume” (cu inițiale posibile și sufix), până la 255 de caractere [observat]; etichetele `AU` și `A1` sunt sinonime, `A2`/`ED` editor [observat]. Autorii se păstrează ca „Nume, Prenume” și se unesc prin ` and `, pentru a trece prin analiza numelor din spec 04 [dedus].
- Titlu: `TI`/`T1` sunt sinonime; `T2` este titlul secundar (revistă, carte, conferință), `T3` terțiar (serie, site) [observat]. Containerul = `T2`, altfel `JF`/`JO`/`JA`/`J1`/`J2` în această ordine [dedus].
- An: `PY` (sinonim `Y1`) este anul sau data publicării primare, patru cifre [observat]. Cititorul ia primele patru cifre consecutive ale valorii; formatul complet al datei (cu bare) nu e descris în sursa citită [dedus; necunoscut].
- DOI: etichetele `DO`, `DI` și `DOI` sunt toate atestate [observat]; valoarea trece prin normalizarea din spec 02.
- `SN` poate fi ISSN, ISBN sau număr de raport/brevet [observat]; cititorul decide prin validarea din spec 02 și, dacă nu se potrivește, o pune în notă [dedus].
- Pagini: `SP` începutul, `EP` sfârșitul (de exemplu 379 și 423 înseamnă paginile 379–423); pentru unele tipuri `SP` și `EP` au alt sens, iar `M2` poate purta o pagină de început [observat]. Cititorul compune intervalul doar când ambele sunt numerice sau cu cifre romane [dedus].
- `UR` și `L2` poartă adrese (repetabile sau separate prin punct și virgulă) [observat]; prima adresă devine url, restul se păstrează în notă [dedus]. `L1` este atașament local [observat].
- `ID` este identificatorul înregistrării, până la 20 de caractere alfanumerice [observat]; devine cheie dacă e nevid și unic, altfel se generează cheia: numele de familie al primului autor în litere mici, fără diacritice, plus anul; la coliziune se adaugă a, b, c… [dedus].
- Tipul `TY` trece prin corespondența din spec 01; cod necunoscut → tip generic [observat pentru codurile din listă; restul dedus].
- Alte etichete (`AB`/`N2` rezumat, `KW`, `PB`, `CY`/`PP`, `VL`, `IS`, `LA`, `N1`/`NO`) se păstrează în câmpurile corespunzătoare dacă intrarea le are, altfel se ignoră [dedus].
- Semnificația unor etichete diferă între versiunile 2001 și 2011 ale specificației și în funcție de tip [observat]; ce se întâmplă cu un fișier care amestecă convențiile este [necunoscut].
- O linie care nu începe cu etichetă validă continuă valoarea anterioară, unită printr-un spațiu [dedus; sursa citită nu descrie continuările — necunoscut].

## Formate / protocoale
- Codificare: UTF-8 cu sau fără BOM; BOM-ul se ignoră [dedus]. Alte codificări (de exemplu cele din exporturi vechi) sunt [necunoscut].
- Etichetă: cele două caractere pot fi litere majuscule sau cifre (de exemplu `C1`, `A2`); sursa citită listează și etichete mai lungi (de exemplu `PMID`, `PMCID`, `DOI`) fără să precizeze cum se formatează [observat; formatul lor este necunoscut]. Cititorul acceptă 2–5 caractere alfanumerice înaintea separatorului „  - ” [dedus].

## Cazuri limită și erori
- Fișier fără nicio înregistrare validă: rezultat gol și avertisment; nu eroare. [dedus]
- Înregistrare fără `ER` la sfârșitul fișierului: acceptată cu avertisment [dedus].
- `TY` lipsă sau nu pe primul loc: înregistrarea e acceptată cu tip generic și avertisment [dedus].
- Autor fără virgulă („Jane Smith”): păstrat întocmai; analiza numelui decide [dedus].
- Valoare goală după separator: eticheta se ignoră. [dedus]
- Fișier binar sau cu octeți invalizi UTF-8: caracterele invalide se înlocuiesc, fără oprire [dedus].

## Teste de acceptanță
1. `TY  - JOUR`, `AU  - Smith, Jane`, `AU  - Doe, John`, `TI  - On proof`, `T2  - Law Rev`, `PY  - 2019`, `DO  - 10.1/x`, `ER  - ` → o intrare cu doi autori, container „Law Rev”, an 2019, doi `10.1/x`, tip `article-journal` (spec 01).
2. Două înregistrări lipite (după `ER  - ` urmează direct `TY  - BOOK`) → două intrări, a doua tip `book`.
3. `SP  - 379` și `EP  - 423` → pagini „379–423”.
4. Înregistrare cu `TY  - THES` fără `ER` la sfârșitul fișierului → intrare tip `thesis`, plus un avertisment.
5. `SN  - 0317-8471` → issn; `SN  - 978-92-95055-12-4` → isbn; `SN  - ZZ-99` → notă (spec 02).
6. Fără `ID`, autor „Smith, Jane”, an 2019, de două ori → chei `smith2019` și `smith2019a`.
7. Fișier scris de programul actual (RIS cu `ID`, `AU`, `TI`, `T2`, `PY`, `DO`) → citit fără pierdere față de intrarea de origine.
8. Linii cu CRLF și linii cu LF în același fișier → aceleași intrări.

## Necunoscute
- Specificația RIS oficială, diferențele 2001/2011 și lista oficială de etichete.
- Formatul complet al datei din `PY` (cu bare) și al etichetelor de peste două caractere.
- Comportamentul continuărilor de linie în exporturi reale.
- Codificarea fișierelor RIS mai vechi.

## Verificare (verificare.md)

Spec: 05-ris-import   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
