# Funcție: Citire BibTeX completă (abrevieri, concatenare, tip de intrare, moștenire prin crossref)

Sursă publică:
- *BibTeXing* (Oren Patashnik), documentația formatului — https://mirrors.ctan.org/biblio/bibtex/base/btxdoc.tex (citit prin oglinda https://ipv4.mirrors.cqupt.edu.cn/ctan/biblio/bibtex/base/btxdoc.tex; secțiunile despre tipuri, câmpuri, `@string`, `#`, `@comment`, `@preamble`, `crossref`)

Stare: candidat · Dimensiune: M · Dependențe: 01 (tipuri), 04 (nume) · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Exporturile din gestionare bibliografică folosesc abrevieri (`@string`), luni ca nume simbolice, valori lipite cu `#` și intrări care moștenesc câmpuri prin `crossref`. Citirea actuală le interpretează greșit sau le pierde, iar titlul, autorii și anul din referințele citate pot ieși trunchiate sau cu nume de abreviere în loc de text.

## Intrări / ieșiri
- Intrare: textul unui fișier `.bib` (UTF-8; alte codificări sunt [necunoscut]).
- Ieșire: lista intrărilor, fiecare cu: cheie, tip (token BibTeX în litere mici), perechi nume-câmp → valoare textuală rezolvată, plus lista avertismentelor (abreviere nedefinită, cheie `crossref` fără țintă, cheie duplicată).
- Câmpurile reținute pe lângă cele existente (autor, an, titlu, container, doi, fișier, notă, url): editor separat de autor, volum, număr, pagini, editură/instituție/școală, adresă, ediție, lună, isbn, issn, tip de lucrare (`type`), `howpublished` [dedus pe baza listei de câmpuri din manual].

## Comportament observabil
- Starea actuală (cod: modulul bib): valoarea fără delimitatori se citește până la următoarea virgulă; `@string` este sărit, deci o abreviere apare literal; după un șir între ghilimele, restul concatenării (`# "…"`) este abandonat; tipul intrării este aruncat; `crossref` este ignorat; `editor` înlocuiește autorul doar când autorul lipsește; `@comment`, `@string` și `@preamble` sunt sărite [observat].
- O valoare de câmp e delimitată de acolade sau de ghilimele duble; numerele ca `year = 1931` pot apărea fără delimitatori [observat].
- `@STRING( nume = "valoare" )` definește o abreviere; definiția poate folosi paranteze rotunde sau acolade [observat pentru paranteze rotunde; acolade dedus din sintaxa generală a intrărilor].
- Operatorul `#` lipește oricâte părți, câte una între fiecare pereche vecină; părțile pot fi șiruri între ghilimele, șiruri între acolade, numere sau nume de abreviere [observat].
- Un nume neîncadrat, ne-numeric, în poziția unei valori este o abreviere; dacă e definită se înlocuiește cu valoarea ei, altfel se păstrează numele ca text și se emite avertisment [dedus; manualul nu descrie comportamentul pentru abrevieri nedefinite — necunoscut].
- Lunile se scriu cu abrevierea standard din trei litere (`jan` … `dec`), de exemplu `jul # "~4"` [observat]. Textul în care se rezolvă abrevierea (nume complet sau abreviat) îl stabilesc stilurile bibliografice, nu manualul [necunoscut]; cele douăsprezece abrevieri se consideră definite implicit, cu numele complet englezesc [dedus].
- `@comment`, `@preamble` și orice text în afara intrărilor sunt ignorate; `%` nu este caracter de comentariu în fișiere `.bib` [observat]. Pentru a scoate o intrare se șterge `@`-ul de la început [observat].
- Câmpuri obligatorii/opționale pe tip sunt definite de manual (de exemplu, `article`: autor, titlu, jurnal, an obligatorii); câmpurile necunoscute sunt ignorate, iar un nume greșit de câmp duce la ignorarea lui [observat]. Lipsa unui câmp obligatoriu nu oprește citirea, produce doar avertisment [dedus].
- Câmpul `crossref` conține cheia intrării citate; intrarea care îl conține moștenește câmpurile pe care nu le are din intrarea țintă [observat]. Manualul impune ca ținta să apară mai târziu în fișier și avertizează că nu se pot înlănțui moșteniri în mod fiabil [observat]. Citirea se face în două treceri (mai întâi toate intrările, apoi moștenirea), deci ordinea din fișier nu mai contează; lanțul se urmează cel mult un nivel [dedus].
- Intrările `@string` definite după folosire: manualul cere definirea înainte de folosire; comportamentul pentru definiții ulterioare este [necunoscut]; recomandare: două treceri, ca la `crossref` [dedus].
- Numele câmpurilor se compară fără diferență de majuscule [dedus; manualul nu spune — necunoscut].

## Formate / protocoale
- Sintaxă de intrare: `@tip{cheie, câmp = valoare, …}` cu acolade, sau cu paranteze rotunde [dedus pe baza exemplului `@STRING(...)`].
- Acoladele din valori controlează protejarea majusculelor și caracterele speciale; curățarea LaTeX (accente, `\&`) rămâne ca în citirea actuală și se aplică valorii rezolvate, după concatenare [dedus].
- Câmpul `file` se păstrează brut (căi Windows), ca azi (cod) [observat].

## Cazuri limită și erori
- Acolade dezechilibrate într-o intrare: intrarea se abandonează cu avertisment, citirea continuă cu următorul `@` [dedus].
- Cheie duplicată: se păstrează prima, se avertizează; manualul nu spune [necunoscut].
- Abreviere care se referă la ea însăși sau lanț de abrevieri: se rezolvă un singur nivel, apoi se oprește cu avertisment [dedus].
- Valoare goală sau câmp fără `=`: câmpul se sare [dedus].
- Intrare `crossref` care arată către o intrare inexistentă: câmpul nu moștenește nimic, avertisment [dedus].

## Teste de acceptanță
1. `@string{tl = "Law Review"} @article{a, journal = tl, year = 2001}` → jurnal „Law Review”.
2. `@article{b, title = "Burden " # {of} # " proof", year = 2001}` → titlu „Burden of proof”.
3. `month = jul # "~4"` → o valoare în care `jul` este înlocuit cu numele lunii (grafia aleasă și documentată de implementare) urmat de „~4”; valoarea nu conține caracterul `#` și nici cuvântul `jul` neschimbat.
4. `@inproceedings{c, crossref = {p1}, title = {T}}` urmat mai jos de `@proceedings{p1, booktitle = {Proc. X}, year = 2005}` → intrarea `c` primește `booktitle` și `year` din `p1`; intrarea păstrează tipul `inproceedings`.
5. `@comment{ignorat} text liber între intrări @misc{d, title = {D}}` → o singură intrare, `d`, tip `misc`.
6. Abreviere nedefinită `journal = zzz` → valoarea „zzz” și un avertisment, nu o eroare.
7. Tipul intrării rămâne disponibil: `@phdthesis{e, …}` → tip `phdthesis` (folosit de spec 01).

## Necunoscute
- Comportament pentru abrevieri nedefinite, cheie duplicată și definire după folosire (manualul tace).
- Sensibilitatea la majuscule pentru tipuri, câmpuri și chei.
- Codificări non-UTF-8 și secvențe LaTeX în afara celor tratate azi.
- Textul exact de ieșire pentru lunile rezolvate.

## Verificare (verificare.md)

Spec: 03-bibtex-values-and-types   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
