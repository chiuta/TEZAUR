# Funcție: Purtare corectă a clientului pentru verificările online (identificare, limite, repetări, cache, retractări)

Sursă publică:
- Crossref, *REST API – Access and authentication* (grupuri polite/publice/Plus, `mailto`, limite, antete, cod 429/403, recomandare de cache) — https://crossref.org/documentation/retrieve-metadata/rest-api/access-and-authentication/ (citit rezumat în sesiune)
- Crossref, documentația REST API (`rest-api-doc`, README: etichetă, limite de rată, paginare cu cursor, filtrul `updates`, `select`, agenție de înregistrare) — https://github.com/CrossRef/rest-api-doc ; text citit din https://raw.githubusercontent.com/CrossRef/rest-api-doc/master/README.md (licență CC BY 4.0)
- Crossref, formatul răspunsurilor (`api_format.md`: câmpul `update-to`, tipurile de actualizare) — https://raw.githubusercontent.com/Crossref/rest-api-doc/master/api_format.md
- *DOI Handbook*, §2.5 (codificarea `#`, `?`, `%` în URL) — https://albany.doi.org/doi_handbook/2_Numbering.html
- Semantica HTTP (antetul `Retry-After`, cod 429) — RFC 9110 și RFC 6585: necitite în sesiune; utilizarea lor aici este [dedus]

Stare: candidat · Dimensiune: M · Dependențe: 02 · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Verificarea referințelor și căutarea de literatură (singurele funcții care ies pe internet) se rulează pe liste întregi de referințe. Azi clientul nu se identifică prin contact decât dacă utilizatorul a setat o variabilă, nu reacționează la limitarea de rată, nu memorează răspunsurile, nu codifică DOI-ul în cale și poate declara „negăsit” un DOI care există la o altă agenție.

## Intrări / ieșiri
- Intrare: lista de intrări bibliografice cu DOI (spec 02) sau titlu; sau o interogare de literatură cu numărul de rezultate dorit.
- Ieșire: pentru fiecare intrare un stat (ok, retractată, metadate diferite, negăsită, înregistrată în altă parte, limitat de rată, blocat, neverificată, eroare) și o notă; plus un rezumat al cererilor trimise, ale celor servite din cache și ale celor refuzate.

## Comportament observabil
- Starea actuală: o cerere are timp de conectare de 8 secunde și de citire de 25; antetul de agent este un text fix cu adresa site-ului proiectului; parametrul de contact se trimite doar dacă variabila de mediu e setată; un cod 404 dă „negăsit”, orice alt cod de stare dă eroare; nu există reîncercări, pauze, cache, nici paginare; DOI-ul intră în calea cererii netranscris; întâi se interoghează OpenAlex, apoi Crossref; retragerea se deduce din `update-to` și `subtype` ale lucrării căutate (cod: modulul de căutări online) [observat].
- Identificare: documentația recomandă un antet de agent care identifică scriptul și oferă un contact `mailto:`, exemplu „NumeScript/1.1 (https://exemplu.org/; mailto:contact@exemplu.org)” [observat]. Cererile HTTPS cu informație de contact merg către un grup rezervat utilizatorilor politicoși; contactul se poate pune și ca parametru `mailto` [observat]. Contactul poate fi o adresă pseudonimă, dar cineva trebuie să citească mesajele [observat]. Clientul trimite contactul în ambele locuri când este configurat și afișează o notă unică dacă nu este (grupul public, cu limite mai mici) [dedus].
- Grupuri și limite (pagina de acces): public 5 cereri și 1 concurentă; politicos 10 și 3; Plus 150 fără limită de concurență; unitatea de timp a limitei nu e dată în pagină [observat]. Antetele `X-Rate-Limit-Limit` și `X-Rate-Limit-Interval` anunță limita în vigoare, intervalul fiind în secunde (de exemplu „50” și „1s”, sau „3000” și „60s”) [observat]. Clientul face cereri una după alta (o singură concurentă) și respectă raportul limită / interval din antete; fără antete, cel mult o cerere pe secundă [dedus].
- Cod 429: limita a fost depășită; se așteaptă puțin, apoi se reia la o rată mai mică sau cu mai puține cereri simultane; pagina nu dă un program de așteptare [observat]. Decizie: se respectă `Retry-After` dacă există, altfel pauze de 2, 4 și 8 secunde, cel mult trei reîncercări; după ele starea este „limitat de rată”, distinctă de „negăsit” și de „eroare” [dedus].
- Cod 403: blocare, rară și manuală, precedată de încercări de contact [observat]. Clientul oprește toate cererile ulterioare din rulare, marchează restul ca „neverificate” și raportează blocarea [dedus].
- Timp de răspuns: documentația cere urmărirea timpilor și retragerea când cresc [observat]. Decizie: dacă un răspuns durează de peste trei ori mediana celor anterioare (minim 5 secunde), se adaugă o pauză de 5 secunde [dedus; pragurile sunt [necunoscut] până la măsurare].
- Cache: documentația cere memorarea răspunsurilor ca să nu se repete aceleași cereri [observat]. Răspunsurile reușite se păstrează 30 de zile, cele „negăsit” 1 zi, cheia fiind punctul final plus DOI-ul canonic (spec 02); un steag explicit ocolește cache-ul [dedus].
- Codificare: DOI-urile și valorile parametrilor se codifică procentual; cursorii la fel, fiindcă pot conține `+` [observat]. Pentru calea cererii: `#` ca `%23`, `?` ca `%3F`, `%` ca `%25` [observat din DOI Handbook].
- Existență: o cerere HEAD pe un obiect individual returnează doar antete și stare (200 există, 404 nu există) [observat]. Aici metadatele sunt necesare (diferențe de titlu și an), deci se folosește GET; HEAD rămâne opțiune pentru verificări doar de existență [dedus].
- Agenția de înregistrare: API-ul funcționează numai pentru DOI Crossref; ruta `/works/{doi}/agency` arată agenția [observat]. Dacă lucrarea nu e găsită în Crossref, se interoghează agenția; dacă este alta (de exemplu pentru seturi de date), starea este „înregistrată în altă parte, neverificabilă aici”, nu „negăsită” [dedus pe baza celor două observații].
- Retragere: formatul răspunsului are `update-to`, listă de actualizări; fiecare conține DOI-ul lucrării actualizate și un tip (de exemplu `retraction`, `correction`) [observat]. Filtrul `updates:{doi}` returnează înregistrările care sunt actualizări editoriale ale DOI-ului cerut [observat]. Deci o lucrare X este retractată dacă există o înregistrare întoarsă de filtrul `updates:X` ale cărei `update-to` conțin DOI-ul X cu tipul `retraction` [dedus]. Verificarea actuală cere ca lucrarea căutată să-și aibă ea însăși `update-to` cu tip `retraction`, adică să fie anunțul de retragere, nu lucrarea retractată [observat în cod; interpretare dedusă din formatul citit]. Dacă răspunsul real mai are și un câmp invers (lucrarea știe că a fost actualizată), este [necunoscut].
- Selectare: parametrul `select` limitează câmpurile întoarse [observat]; se cer doar DOI, titlu, autori, data, titlul containerului, tipul, subtipul, `update-to` și numărul de citări [dedus].
- Paginare: implicit 20 de rezultate, maximum 1000 pe cerere (`rows`); `offset` limitat la 10.000 pentru lucrări; pentru mai mult se folosește `cursor=*` și valoarea `next-cursor` din răspuns; dacă lista are mai puține elemente decât `rows`, s-a ajuns la capăt [observat]. Căutarea de literatură cere azi cel mult 50 de rezultate (cod) [observat]; paginarea cu cursor se folosește doar pentru cereri peste `rows` [dedus].
- Notificarea de confidențialitate din fața fiecărei rulări online (ce se trimite) rămâne ca azi (documentația proiectului) [observat].

## Formate / protocoale
- Răspuns Crossref: obiect cu `message`; listele au `items`, `total-results` și, la cursor, `next-cursor` [observat pentru `items` și `next-cursor`; `total-results` necunoscut în extras].
- Tip media pentru răspunsuri: `application/vnd.crossref-api-message+json` [observat]; clientul acceptă și `application/json` [dedus].
- Identificare: antet de agent și, dacă există, parametrul `mailto`. [dedus]

## Cazuri limită și erori
- Fără rețea: starea „eroare” cu motivul; nu se reîncearcă la nesfârșit [dedus].
- Răspuns neinterpretabil (nu e JSON): „eroare”, nu „negăsit” [dedus].
- Antet de limită cu valoare ilizibilă: se folosește implicitul de o cerere pe secundă [dedus].
- Lucrare fără DOI: căutare pe titlu, ca azi; acceptată doar cu acoperire reciprocă ≥ 0,8 (cod) [observat].
- Valori Plus (token) nu sunt folosite; clientul nu trimite niciodată jetoane de autentificare [dedus].

## Teste de acceptanță
(cu un server local care imită API-ul; fiecare caz se verifică din textele citite)
1. Contact configurat → cererea poartă `mailto:` în antetul de agent și parametrul `mailto`; fără contact → grupul public și nota unică.
2. DOI `10.1000/a#b?c` → calea cererii conține `a%23b%3Fc`.
3. Răspunsuri cu `X-Rate-Limit-Limit: 1` și `X-Rate-Limit-Interval: 1s` → cinci cereri durează cel puțin patru secunde și nu se suprapun.
4. Un 429 urmat de 200 → rezultatul reușit; patru 429 la rând → starea „limitat de rată”, nu „negăsit”.
5. Un 403 → cererile rămase nu se mai trimit; intrările rămase au starea „neverificate”.
6. 404 pe `/works/{doi}` și agenție diferită de Crossref → „înregistrată în altă parte”; 404 și agenție Crossref → „negăsită”.
7. Răspuns la `updates:X` cu o înregistrare ale cărei `update-to` conțin `{DOI: X, type: retraction}` → X retractat; cu `type: correction` → nu e retractat.
8. A doua verificare a aceluiași DOI în 30 de zile → nicio cerere de rețea; cu steagul de ocolire → cerere nouă.
9. Cerere de 25 de rezultate cu `rows` 10 → trei pagini prin `next-cursor`; oprire când pagina are mai puțin de 10 elemente.

## Necunoscute
- Dacă API-ul live adaugă un câmp invers pentru lucrările retractate (documentul citit nu-l descrie).
- Unitatea de timp pentru limitele din pagina de acces și valorile curente (documentele citite diferă în detalii; cel mai nou e pagina de acces).
- Textul RFC 9110/6585 pentru `Retry-After` și 429.
- Condițiile de utilizare ale OpenAlex (nu s-a citit nicio documentație OpenAlex; această specificație acoperă purtarea față de Crossref).

## Verificare (verificare.md)

Spec: 12-crossref-client-conduct   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
