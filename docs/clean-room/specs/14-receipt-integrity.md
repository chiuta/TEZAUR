# Funcție: Chitanțe verificabile independent (serializare canonică JSON și identitate criptografică a documentelor)

Sursă publică:
- RFC 8785, *JSON Canonicalization Scheme (JCS)*, A. Rundgren, B. Jordan, S. Erdtman, iunie 2020 — https://www.rfc-editor.org/rfc/rfc8785 (citit rezumat în sesiune: secțiunile 3.1, 3.2.1–3.2.4, exemplul de sortare, exemplul complet, Anexa B cu serializarea numerelor)
- FIPS 180-4, *Secure Hash Standard* (SHA-256) și RFC 2104 / RFC 4231 (HMAC-SHA-256 și vectori de test) — necitite în sesiune; vectorii de mai jos sunt din cunoștințe generale: [dedus]

Stare: candidat · Dimensiune: M · Dependențe: niciuna · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
O chitanță de răspuns este dovada, pentru comisie sau pentru autor peste ani, că întrebarea, modelul, răspunsul și pasajele nu au fost modificate. Azi verificarea depinde de comportamentul unei biblioteci Rust (cum ordonează și scrie JSON-ul) și de o amprentă de 64 de biți necriptografică pentru documente; un alt program nu poate recalcula amprenta în mod garantat, iar identitatea documentelor este slabă.

## Intrări / ieșiri
- Intrare: conținutul unei chitanțe (câmpuri: format, creat, instrument, model, mod, întrebare, amprenta răspunsului, amprenta indexului, lista pasajelor cu cale, titlu, amprenta documentului, pagină, amprenta pasajului), eventual cheia secretă a utilizatorului.
- Ieșire: aceeași chitanță cu o amprentă SHA-256 și, dacă există cheie, un cod HMAC-SHA-256; la verificare: lista verificărilor trecute sau eșuate.
- Format nou: `tezaur-receipt/2`; chitanțele `tezaur-receipt/1` se verifică în continuare cu procedura veche [dedus].

## Comportament observabil
- Starea actuală: amprenta chitanței = SHA-256 al serializării JSON produse de biblioteca standard a programului pentru corpul chitanței, fără câmpurile `digest` și `hmac`; HMAC-ul se calculează peste textul hexazecimal al amprentei, cu cheia dintr-o variabilă de mediu; amprenta indexului = SHA-256 al amprentelor documentelor sortate și unite prin virgulă; amprenta documentului este FNV-1a pe 64 de biți, scrisă în 16 caractere hexazecimale; amprenta pasajului și a răspunsului sunt SHA-256 (cod: modulul de audit și cel de indexare, funcțiile de chitanță; biblioteca de JSON a programului, declarată în fișierul de dependențe fără caracteristici suplimentare) [observat].
- Ordinea cheilor în serializarea actuală este cea a bibliotecii (implicit, ordine alfabetică după octeții UTF-8) [dedus din comportamentul implicit al bibliotecii; dacă o altă dependență activează ordinea de inserție este necunoscut]. Pentru câmpurile folosite azi (nume ASCII fixe, valori șir și numere întregi) rezultatul coincide cu cel al JCS [dedus]; coincidența este un accident de implementare, nu un contract.
- JCS (RFC 8785) fixează reprezentarea: fără spații între jetoane; literalele `null`, `true`, `false` neschimbate; șiruri între ghilimele cu doar aceste scăpări: `\b \t \n \f \r`, `\"`, `\\` și `\u00xx` cu hexazecimale mici pentru restul caracterelor U+0000–U+001F; toate celelalte caractere scrise direct; surogat singur = eroare; numere în forma ECMAScript (cu regula „Note 2”); NaN și Infinity = eroare; proprietățile fiecărui obiect, recursiv, sortate după numele necodificat comparat ca unități de cod UTF-16 fără semn, independent de limbă; ordinea elementelor din liste nu se schimbă; ieșire finală în UTF-8 [observat].
- Restricții de intrare JCS: nume de proprietăți duplicate interzise; șiruri exprimabile în Unicode; numere ca IEEE 754 dublă precizie (numerele mai mari se pun în șiruri); șirurile nu se normalizează Unicode [observat].
- Diferența de ordonare față de ordinea octeților UTF-8: pentru numele `"😀"` (emoji, U+1F600) și `"דּ"` (U+FB33), JCS pune emoji-ul înaintea lui U+FB33 (unități de cod UTF-16: D83D înainte de FB33), iar ordinea octeților UTF-8 le inversează [observat pentru ordinea JCS din exemplul RFC; inversarea UTF-8 dedusă din codificare (U+FB33 = EF AC B3, U+1F600 = F0 9F 98 80)].
- Procedura v2 [dedus]: (1) se construiește corpul chitanței fără `digest` și `hmac`; (2) se serializează conform JCS; (3) amprenta = SHA-256 al acestor octeți, în hexazecimal cu litere mici; (4) dacă există cheie, `hmac` = HMAC-SHA-256 cu cheia peste aceiași octeți canonici (nu peste textul hexazecimal); (5) chitanța finală = corpul + `digest` + eventual `hmac`.
- Corpul v2 poate conține numai șiruri, numere întregi cu valoare absolută sub 2^53, liste și obiecte; numerele cu parte fracționară se pun în șiruri, ca să nu depindă de formatul numerelor [dedus pe baza recomandării din RFC pentru valori peste dublă precizie].
- Identitatea documentelor în v2: fiecare pasaj numește documentul prin SHA-256 al octeților fișierului la momentul ingestiei (`doc_sha256`), nu prin FNV [dedus]. Indexul păstrează amprenta de 64 de biți pentru detectarea schimbărilor (nu se schimbă) și o amprentă SHA-256 aparte, calculată la ingestie [dedus; formatul indexului se extinde, compatibilitatea înapoi a indexului este [necunoscut]]. Pentru documente ingerate înainte (doar FNV disponibil), chitanța scrie `doc_sha256` absent și marchează documentul ca „identitate slabă (64 de biți, necriptografică)” [dedus].
- Amprenta indexului v2 = SHA-256 al amprentelor SHA-256 ale documentelor sortate, unite prin virgulă, doar dacă toate documentele au SHA-256; altfel se omite și se notează motivul [dedus].
- Verificarea v2: se recalculează amprenta peste forma canonică a chitanței citite (re-serializată cu JCS, indiferent cum e formatată în fișier), se compară; se verifică HMAC dacă există cheie; opțional răspunsul (amprenta lui) și pasajele din index, ca azi [observat pentru verificările existente; recalcularea canonică dedusă]. O chitanță cu proprietăți duplicate sau numere fracționare în corp este respinsă ca invalidă [dedus].
- Chitanțele nu sunt semnături de identitate și nu dovedesc că răspunsul e adevărat; textul de ajutor rămâne neschimbat (documentația proiectului) [observat].

## Formate / protocoale
- Chitanță: obiect JSON UTF-8, orice spațiere și orice ordine a cheilor în fișier; amprenta se calculează numai peste forma canonică [dedus].
- `digest`: 64 de caractere hexazecimale mici; `hmac`: 64 de caractere hexazecimale mici [dedus].
- `format`: `tezaur-receipt/2` pentru chitanțele noi; verificarea alege procedura după acest câmp [dedus].

## Cazuri limită și erori
- Șir cu surogat singur în întrebare (intrare coruptă): nu se emite chitanța; eroare explicită [observat pentru regula JCS; aplicare dedusă].
- Cheie secretă goală: ca lipsa ei (ca azi) [observat în cod].
- Chitanță v1 cu `hmac` și fără cheia utilizatorului: verificarea notează că HMAC-ul nu a putut fi verificat, fără eșec (ca azi) [observat în cod].
- Câmpuri necunoscute într-o chitanță v2: păstrate și incluse în amprentă; nu sunt ignorate [dedus].

## Teste de acceptanță
1. Ordinea JCS: un obiect cu cheile `"\r"`, `"1"`, `"\u0080"`, `"ö"`, `"€"`, `"😀"`, `"דּ"` se serializează cu cheile exact în această ordine (cea din exemplul RFC 8785 §3.2.3); serializarea octeților UTF-8 ordonați altfel dă o amprentă diferită.
2. Exemplul complet din RFC 8785 (secțiunea 3.2.2, intrarea) dă exact: `{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}`.
3. Numere (dacă implementarea acceptă numere generale): dubla cu octeții hexazecimali `4340000000000000` → `9007199254740992`; `444b1ae4d6e2ef50` → `1e+21`; `3eb0c6f7a0b5ed8d` → `0.000001`; `8000000000000000` (minus zero) → `0`; `7ff0000000000000` (infinit) → eroare.
4. Chitanță emisă și apoi reformatată (indentare, ordine diferită a cheilor) → verificare reușită; modificarea unui singur caracter din răspuns, din întrebare sau dintr-o amprentă de pasaj → verificare eșuată.
5. Cu cheie: HMAC corect → trece; cheie greșită → eșec; fără cheie → „neverificat”, nu eșec.
6. Chitanță `tezaur-receipt/1` emisă de versiunea actuală → verificată în continuare cu procedura veche.
7. [necesită FIPS 180-4, necitit] SHA-256 al șirului „abc” este `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`.
8. [necesită RFC 4231, necitit] HMAC-SHA-256 cu cheia „Jefe” și datele „what do ya want for nothing?” este `5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843`.
9. Document ingerat înainte de schimbare (fără SHA-256) → chitanța îl marchează „identitate slabă” și omite amprenta indexului.

## Necunoscute
- Dacă ordinea alfabetică a cheilor din serializarea actuală este garantată în toate construcțiile programului.
- Compatibilitatea înapoi a formatului de index cu o amprentă SHA-256 suplimentară.
- Valoarea juridică a unei chitanțe (nu este semnătură; marcarea temporală de către o terță parte, ex. RFC 3161, nu face parte din această specificație).
- Textul FIPS 180-4 și RFC 4231 (necitite; testele 7–8 sunt condiționate).

## Verificare (verificare.md)

Spec: 14-receipt-integrity   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
