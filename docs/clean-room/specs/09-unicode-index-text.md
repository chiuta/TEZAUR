# Funcție: Text de indexare conform Unicode (normalizare, pliere, segmentare în cuvinte)

Sursă publică:
- *Unicode Standard Annex #15: Unicode Normalization Forms*, rev. 56 (Unicode 16.0.0) — https://www.unicode.org/reports/tr15/ ; versiunea citită: https://www.unicode.org/reports/tr15/tr15-56.html (definițiile NFD/NFC/NFKD/NFKC, ligaturi, stabilitate, idempotență; citit parțial, rezumat)
- *Unicode Standard Annex #29: Unicode Text Segmentation*, secțiunea „Word Boundaries” — https://www.unicode.org/reports/tr29/ ; versiunea citită: https://www.unicode.org/reports/tr29/tr29-44.html (rev. 44, ciornă pentru Unicode 16.0.0, 2024-06-06; regulile WB3–WB999 și listele MidLetter, MidNumLet, MidNum, Single_Quote; citit parțial, rezumat, regula WB14 nu a apărut în extras)
- Wikipedia, „Ș” — https://en.wikipedia.org/wiki/%C8%98 (doar punctele de cod U+0218/U+0219 și faptul că S cu sedilă există din Unicode 1.1.0)

Stare: candidat · Dimensiune: M · Dependențe: niciuna (10 depinde de ea) · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Căutarea și verificarea citărilor depind de cum se desparte textul în cuvinte și cum se aduc literele la o formă comună. Tabela actuală de pliere a diacriticelor este scrisă de mână și incompletă (ligaturile, litere cu diacritice rare, forme descompuse din PDF-uri nu sunt acoperite). PDF-urile și EPUB-urile aduc des ligaturi (ﬁ, ﬂ), litere cu accent în formă descompusă și cratime moi.

## Intrări / ieșiri
- Intrare: un text (pasaj de document sau interogare).
- Ieșire: lista ordonată de „jetoane de indexare”, fiecare cu poziția cuvântului din care provine (poziția numără toate cuvintele, inclusiv cele prea scurte ca să fie indexate).
- Aceeași funcție se aplică identic textului indexat și interogării.

## Comportament observabil
- Starea actuală: textul se parcurge caracter cu caracter; fiecare caracter se trece în minuscule, se pliază printr-o tabelă fixă (diacritice românești și franceze uzuale, ñ, ß→s), orice caracter ne-alfanumeric desparte jetonul, jetoanele de sub două caractere se elimină, fiecare jeton se trunchiază la 7 caractere (stemming prin prefix); la ingestie, împărțirea de rând cu cratimă („-” urmat de rând nou) se lipește (cod: modulele de index și de ingestie) [observat].
- Etapa 1, normalizare: textul se aduce la NFKD (descompunere de compatibilitate) [dedus]. NFKC/NFKD desfac ligaturile de compatibilitate (ﬁ devine „f” + „i”); NFC/NFD nu o fac, pentru că maparea e de compatibilitate, nu canonică [observat]. Orice transformare de normalizare este idempotentă, iar un șir normalizat rămâne normalizat în versiunile viitoare [observat].
- Etapa 2, pliere a diacriticelor: după NFKD se elimină semnele combinate de categoria generală Mn (accente, virgulă dedesubt, sedilă, brevis) [dedus]. Astfel ă, â, î, é, ü etc. devin litera de bază. Litera S cu virgulă dedesubt (U+0218/U+0219) și S cu sedilă (U+015E/U+015F), folosită istoric în texte românești [observat pentru existența ambelor], dau același rezultat; la fel T cu virgulă dedesubt și T cu sedilă [dedus; descompunerile canonice ale acestor patru caractere trebuie confirmate în baza de date Unicode — necunoscut].
- Literele fără descompunere canonică (ø, đ, ł, æ, œ, ß) nu sunt atinse de etapa 2; tratamentul lor (de exemplu ß → „ss”, æ → „ae”) rămâne o tabelă mică, explicit documentată, extinsă față de azi [dedus; pentru ß, practica actuală „s” este [necunoscut] dacă e dorită].
- Etapa 3, minuscule: trecere în minuscule după standardul Unicode de pliere a majusculelor (nu doar minuscule simple) [dedus; tabelul de pliere nu a fost citit — necunoscut].
- Etapa 4, caractere de format: cratima moale (U+00AD) și alte caractere de format se elimină din interiorul cuvântului; segmentarea Unicode ignoră caracterele Format și Extend, atașându-le caracterului precedent [observat pentru regula de ignorare, WB4; clasificarea U+00AD drept Format este dedusă].
- Etapa 5, segmentare în cuvinte după regulile standard: nu se desparte între două litere (WB5), nici între litere separate de un semn din MidLetter sau MidNumLetQ (WB6, WB7), nici între cifre (WB8), între literă și cifră (WB9, WB10), nici între cifre separate de MidNum sau MidNumLetQ (WB11, WB12); caracterul `_` și cele de tip ExtendNumLet leagă (WB13a, WB13b); CRLF nu se desparte (WB3); la orice alt loc se desparte (WB999) [observat].
- Setul MidNumLet include U+002E (punct), U+2018, U+2019, U+2024, U+FE52, U+FF07 și U+FF0E; Single_Quote este doar U+0027; MidLetter include U+003A (două puncte), U+00B7, U+0387, U+055F, U+05F4, U+2027, U+FE13, U+FE55, U+FF1A; MidNum conține semnele cu Line_Break = Infix_Numeric plus U+066C, U+FE50, U+FE54, U+FF0C, U+FF1B, fără U+003A, U+FE13 și U+002E [observat în versiunea 44]. Definiția „MidNumLetQ” (MidNumLet sau Single_Quote) nu a apărut în extras [dedus; necunoscut]. Conținutul acestor liste depinde de versiunea Unicode [necunoscut pentru versiunile ulterioare].
- Consecințe: „l’été” (cu U+2019) este un singur cuvânt, „3.5” este un singur cuvânt, „state-of-the-art” sunt patru cuvinte (cratima U+002D nu apare în nicio listă Mid citită) [observat pentru listele citite; absența cratimei din celelalte liste este dedusă].
- Jetoane de indexare: pentru fiecare cuvânt se indexează forma întreagă fără semnele interne (apostrof, punct) și, când cuvântul conține astfel de semne, și părțile lui separate [dedus; păstrează regăsirea de azi, unde apostroful desparte]. La cuvintele formate numai din cifre și semne interne, semnul se păstrează, ca „3.5” să nu devină „35” [dedus]. Segmentele fără nicio literă sau cifră (spații, semne de punctuație) nu produc jetoane [dedus].
- Jetoanele cu mai puțin de două caractere se elimină și trunchierea la 7 caractere rămâne ca azi, dar poziția cuvântului se păstrează ca poziție în text [dedus; necesar pentru spec 10].
- Lipirea cratimelor de capăt de rând (înainte de rupere) rămâne, dar se aplică doar între două litere; „-” urmat de rând nou și de o cifră sau o majusculă nu se lipește [dedus]. Ce fac PDF-urile reale cu cratimele de compuse („state-of-the-art” rupt la rând) este [necunoscut].
- Cache-ul de index se invalidează la schimbarea acestei funcții: cheia lui poartă un număr de versiune al regulilor [dedus]. Textul salvat în index nu se schimbă, doar jetoanele derivate se recalculează [observat: cache-ul BM25 e reconstruit din text; cod: modulul de index].

## Formate / protocoale
- Normalizare și segmentare conform versiunii Unicode a bibliotecii folosite; versiunea se înregistrează lângă numărul de versiune al regulilor [dedus].
- Ieșirea este codificată UTF-8; nu se aplică NFC la textul stocat (doar la jetoane) [dedus].

## Cazuri limită și erori
- Scripturi fără spații între cuvinte (chineză, japoneză, thailandeză): segmentarea standard cere dicționar sau reguli suplimentare pentru acestea; neacoperit de funcție [dedus; sursa citită nu detaliază — necunoscut].
- Text cu secvențe invalide (surogate singure): înlocuite cu U+FFFD înainte de normalizare [dedus].
- Șir gol: nicio ieșire. [dedus]
- Emoji și simboluri: nu produc jetoane. [dedus]

## Teste de acceptanță
1. „ﬁnal” (U+FB01 + „nal”) și „final” dau același jeton „final”.
2. „é” (U+00E9) și „e” + U+0301 dau același jeton „e”; normalizarea aplicată de două ori dă același rezultat ca o singură aplicare.
3. „l’été” cu U+2019 este un singur cuvânt; jetoane: „lete” și „ete” (poziție 0), nu „l” (sub două caractere).
4. „state-of-the-art” → patru cuvinte pe pozițiile 0–3; „of” intră ca jeton, „the” ca jeton.
5. „3.5” este un singur cuvânt; jeton „3.5” (cu punct păstrat în jetonul cifric) [forma jetonului numeric este decizie de proiectare].
6. „con­stitution” cu U+00AD între „con” și „stitution” dă jetonul „constitution”.
7. [necesită baza de date Unicode, necitită] „Ștefan” (U+0218), „Ştefan” (U+015E) și „stefan” dau același jeton.
8. Căutarea interogării „ﬁnal” găsește pasajul care conține „final” (și invers), în același clasament.

## Necunoscute
- Descompunerile canonice exacte ale S/T cu virgulă și cu sedilă (nu au fost citite).
- Tabelul oficial de pliere a majusculelor (nu a fost citit).
- Regula WB14 și definiția MidNumLetQ din ediția citită.
- Efectul asupra calității de regăsire: se măsoară cu comanda de evaluare a căutării pe un set real de întrebări înainte de adoptare [necunoscut].

## Verificare (verificare.md)

Spec: 09-unicode-index-text   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
