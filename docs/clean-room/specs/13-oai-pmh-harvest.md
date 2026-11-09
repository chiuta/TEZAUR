# Funcție: Recoltare de metadate bibliografice din arhive instituționale (protocol OAI-PMH, Dublin Core necalificat)

Sursă publică:
- *The Open Archives Initiative Protocol for Metadata Harvesting*, versiunea 2.0 (data exactă a ediției citite nu a fost afișată în extras: necunoscut) — http://www.openarchives.org/OAI/openarchivesprotocol.html ; text citit prin oglinda https://openarchives-new.library.cornell.edu/OAI/2.0/openarchivesprotocol.htm (secțiunile 2.5.1, 3.1–3.6, 4.1–4.5, 5; citit parțial: extrasul se oprește în mijlocul secțiunii 4.5 ListRecords, iar 4.6 ListSets nu a fost citită; rezumat)
- *Dublin Core Metadata Element Set* (cele 15 elemente) — https://www.dublincore.org/specifications/dublin-core/dces/ (necitit în sesiune; lista elementelor de mai jos provine din cunoștințe generale: [dedus])
- Pentru identificatori: spec 02; pentru purtarea clientului HTTP: spec 12

Stare: candidat · Dimensiune: L · Dependențe: 01, 02, 12 · Data: 2026-11-09

Marcaje: [observat] = afirmat în sursa publică citită în sesiune sau constatat prin citirea repo-ului (marcat „cod”); [dedus] = inferență sau decizie a autorului specificației; [necunoscut] = nestabilit.

## Scop
Tezele și articolele din arhive universitare (inclusiv repozitorii de teze) se pot descoperi azi doar prin căutarea de literatură în două servicii mari. Protocolul OAI-PMH permite să se interogheze direct o arhivă instituțională aleasă de utilizator și să se importe metadatele ei ca referințe, fără cont și fără serviciu terț.

## Intrări / ieșiri
- Intrare: adresa de bază a unei arhive, formatul de metadate (implicit `oai_dc`), filtre opționale (set, de la, până la), un plafon de înregistrări.
- Ieșire: intrări bibliografice (aceeași formă ca la spec 03/05/06), o listă a înregistrărilor șterse de arhivă și o stare de recoltare reutilizabilă pentru recoltări incrementale.
- Rulare numai la cerere explicită, cu aceeași notă de confidențialitate ca la celelalte funcții online (documentația proiectului: doar două funcții ies pe internet) [observat].

## Comportament observabil
- Starea actuală: nu există nicio funcție de recoltare; căutarea online folosește două servicii cu API propriu (cod: modulul de căutări online) [observat].
- Protocolul are șase verbe: GetRecord, Identify, ListIdentifiers, ListMetadataFormats, ListRecords, ListSets [observat]. Funcția folosește Identify (granularitate și politica de înregistrări șterse), ListMetadataFormats (confirmă `oai_dc`) și ListRecords [dedus]; GetRecord pentru reluarea unei singure înregistrări [dedus].
- Cerere ListRecords: argument obligatoriu `metadataPrefix` (dacă nu se folosește `resumptionToken`), opționale `from`, `until`, `set`; `resumptionToken` este exclusiv: singurul argument pe lângă `verb` [observat].
- Transport: arhivele trebuie să accepte GET (argumente în șirul de interogare, cu caracterele rezervate `/ ? & = :` codificate procentual) și POST (`application/x-www-form-urlencoded`); răspunsurile au tip `text/xml`; un cod 503 cu `Retry-After` cere așteptare înainte de reîncercare; un 302 poate redirecționa; se acceptă codarea de conținut „identity” [observat]. Clientul folosește GET, urmează cel mult trei redirecționări și, la 503, așteaptă perioada indicată (valoare în secunde; forma dată-HTTP a antetului este [necunoscut] din textele citite) [dedus].
- Plic de răspuns: element rădăcină `OAI-PMH`; primul copil `responseDate` (UTC), al doilea `request` (ecou al adresei de bază și argumentelor), al treilea fie `error`, fie elementul numit după verb [observat]. În ListRecords fiecare `record` are `header` obligatoriu (`identifier`, `datestamp`, zero sau mai multe `setSpec`), `metadata` opțional și `about` repetabil [observat].
- Înregistrare ștearsă: antetul poartă `status="deleted"`, iar înregistrarea nu are `metadata` sau `about`; prezența lor în răspuns depinde de politica declarată (`no`, `persistent`, `transient`) [observat]. Înregistrările șterse nu devin referințe; se listează separat [dedus].
- Jeton de reluare: dacă lista e incompletă, răspunsul are un `resumptionToken` nevid, care se trimite înapoi ca singur argument; ultima porțiune întoarce elementul gol; atribute opționale: `expirationDate`, `completeListSize` (poate fi aproximativ), `cursor` (de la 0) [observat]. Jetonul este opac și se codifică procentual la reutilizare [observat]. Un jeton invalid sau expirat dă `badResumptionToken`; dacă mulțimea s-a schimbat mult, arhiva poate returna aceeași eroare [observat].
- Reluare după eșec: la `badResumptionToken` sau eșec de rețea, recoltarea reîncepe cu aceleași filtre și `from` egal cu cea mai mare dată-marcă deja primită (granularitatea arhivei) [dedus pe baza secțiunii 3.5.1, citită doar din titlu și rezumat].
- Date-marcă: toate valorile sunt UTC, cu `Z` când e prezentă ora; granularitatea de zi `YYYY-MM-DD` e obligatorie, cea de secundă `YYYY-MM-DDThh:mm:ssZ` e opțională și anunțată în Identify; `from` și `until` au aceeași granularitate; o granularitate mai fină decât cea suportată dă eroare [observat]. Clientul alege granularitatea din Identify [dedus].
- Erori: `badArgument` (argumente ilegale, lipsă sau repetate, sintaxă greșită, `from` după `until`); `badResumptionToken`; `badVerb`; `cannotDisseminateFormat` (format nesuportat); `idDoesNotExist`; `noRecordsMatch` (combinația de argumente dă listă goală); `noMetadataFormats`; `noSetHierarchy` (arhiva nu are seturi) [observat]. `noRecordsMatch` înseamnă zero rezultate, nu eșec [dedus din definiție]. Care cod HTTP însoțește un `error` din plic este [necunoscut] din textele citite; clientul tratează orice răspuns 200 cu `error` după cod [dedus].
- Dublin Core: toate arhivele trebuie să ofere Dublin Core necalificat, cu prefixul rezervat `oai_dc` și spațiul de nume `http://www.openarchives.org/OAI/2.0/oai_dc/` [observat].
- Corespondența cu o intrare bibliografică [dedus; lista de 15 elemente DC: title, creator, subject, description, publisher, contributor, date, type, format, identifier, source, language, relation, coverage, rights]:
  - title → titlu; creator (repetat) → autori, în ordine; contributor → editori;
  - date → anul (primele patru cifre ale primei valori; formatul datelor DC în repozitorii reale este [necunoscut]);
  - identifier (repetat) → fiecare valoare trece prin spec 02: DOI → doi; adresă `http(s)` → url (prima); ISBN/ISSN → câmpurile omonime; valoarea OAI proprie a înregistrării (identificatorul din antet) nu e DOI;
  - type → tipul lucrării: dacă valoarea conține „thesis”, „dissertation”, „teză” sau „doctor” → teză; „article” → articol de revistă; „book” → carte; „report” → raport; altfel document generic (spec 01);
  - publisher → editura / școala; source → container; description → rezumat; subject → cuvinte-cheie; language → limba; relation, coverage, format, rights → note, doar dacă sunt scurte;
  - elementele sunt repetabile și pot lipsi toate [dedus; repetarea e atestată pentru `setSpec`, `about`, iar pentru DC este dedusă].
- Cheie de intrare: ca la spec 05 (nume + an + literă) [dedus].
- Stare de recoltare: pentru fiecare pereche (adresă de bază, set), cea mai mare dată-marcă primită; la rularea următoare `from` = acea dată, ca să se aducă doar înregistrările modificate [dedus]; arhiva decide ce înseamnă modificare (și șterge) în funcție de politica ei [observat].
- Plafon: implicit 1000 de înregistrări pe rulare, ca o recoltare accidentală de milioane de înregistrări să nu pornească [dedus].
- Cereri secvențiale, cu pauză între cereri și identificare cu contact, conform spec 12 [dedus].
- Siguranța XML: documentul se analizează fără a rezolva entități externe și fără a expanda entități în mod nelimitat; răspunsurile peste 16 MB se resping [dedus; specificația XML nu a fost citită].

## Formate / protocoale
- Cerere: adresă de bază + `?verb=ListRecords&metadataPrefix=oai_dc[&set=…][&from=…][&until=…]`, sau `?verb=ListRecords&resumptionToken=…` [observat].
- Codificare: UTF-8; caracterele rezervate se codifică procentual [observat].
- Starea de recoltare: fișier mic lângă index, cu adresa de bază, setul și data-marcă [dedus].

## Cazuri limită și erori
- `noRecordsMatch` la prima cerere: rezultat gol, rulare reușită. [dedus]
- `cannotDisseminateFormat`: oprire cu mesaj; se afișează formatele din ListMetadataFormats [dedus].
- `noSetHierarchy` când s-a cerut `set`: oprire cu mesaj. [dedus]
- Arhivă care nu răspunde cu XML: eroare, nu zero rezultate. [dedus]
- Fără `completeListSize`: nu se afișează procent de progres [dedus].
- Înregistrare fără titlu: se importă numai dacă are cel puțin un identificator; altfel se sare [dedus].
- Aceeași lucrare cu DOI deja în bibliografie: nu se dublează (spec 07) [dedus].

## Teste de acceptanță
(cu un server local care imită o arhivă; fiecare caz urmează textul protocolului)
1. ListRecords cu `metadataPrefix=oai_dc` → un răspuns cu două `record`, `resumptionToken` nevid; a doua cerere conține doar `verb` și `resumptionToken`; a treia întoarce `resumptionToken` gol → recoltare încheiată cu toate înregistrările.
2. Înregistrare cu `<header status="deleted">` → nu devine referință; apare în lista de înregistrări șterse.
3. Răspuns cu `<error code="noRecordsMatch">` → zero intrări, cod de ieșire de succes.
4. Răspuns cu `<error code="badResumptionToken">` în mijlocul recoltării → recoltarea reîncepe cu `from` = cea mai mare dată-marcă primită și nu dublează înregistrările deja importate.
5. Răspuns HTTP 503 cu `Retry-After: 2` → clientul așteaptă cel puțin două secunde, apoi reia.
6. Înregistrare `oai_dc` cu `dc:title`, două `dc:creator`, `dc:date` „2019-05-03”, `dc:identifier` „https://doi.org/10.1000/xyz” și `dc:type` „Doctoral thesis” → intrare tip teză, doi `10.1000/xyz`, an 2019, doi autori în ordine.
7. Cerere cu `from=2020-01-01` și `until=2020-06-30T00:00:00Z` (granularități diferite) → refuzată înainte de trimitere (`badArgument` potențial).
8. Parametrii din cerere au caracterele rezervate codificate procentual (un `set` conținând `:`).

## Necunoscute
- Lista oficială a celor 15 elemente Dublin Core și semantica `dc:type`, `dc:date` în arhive reale (nu s-au citit).
- Codul HTTP pentru erori de aplicație din plic.
- Formatul `Retry-After` când este dată-HTTP.
- Comportament pe arhive reale (nicio arhivă nu a fost interogată; protocolul a fost citit din documentație).
- Secțiunea 4.6 (ListSets), 3.5.1 și 4.5 în întregime.

## Verificare (verificare.md)

Spec: 13-oai-pmh-harvest   Data: 2026-11-09

- [x] Nicio linie de cod, pseudocod sau asamblare copiată (verificat: fără blocuri de cod în fișier; formulele sunt cele ale standardelor citate, enunțate în cuvinte)
- [x] Niciun nume intern de funcție, variabilă sau clasă din țintă (nu există țintă terță; verificat: nicio denumire de funcție a proiectului nu apare, doar nume de module în limbaj natural)
- [x] Niciun text de UI, asset sau branding din țintă (verificat: nu sunt citate texte de interfață)
- [x] Algoritmii sunt descriși prin intrare → ieșire, nu prin structura codului țintei
- [x] Afirmațiile sunt marcate [observat] / [dedus] / [necunoscut] (verificat automat în secțiunile Comportament observabil, Formate / protocoale și Cazuri limită; Scop și Intrări / ieșiri repetă afirmații marcate acolo; testele moștenesc marcajul comportamentului pe care îl verifică)
- [ ] Licența/EULA țintei verificată și notată în jurnal (licențele surselor sunt notate în JURNAL.md, dar cel puțin una nu a fost verificată în sesiune; rămâne nebifat)
- [ ] Specificația nu depinde de brevete sau secrete comerciale (nu s-a făcut nicio căutare de brevete; sursele sunt documente publice, dar absența brevetelor nu a fost verificată)
- [ ] Aprobată de mine înainte de predare

Doar după bifarea tuturor: copiază specificația în `3-implementare/spec/`.
