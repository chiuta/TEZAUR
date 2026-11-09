# Teme analizate și neincluse în lotul 1 (2026-11-09)

Motivele sunt cele reale ale sesiunii: fie lipsa unui standard sau a unei metode publicate citite, fie dimensiunea, fie dependența de ceva proprietar. „Necitit” înseamnă că sursa publică există, dar nu a fost citită în această sesiune (accesul de rețea al sesiunii a permis doar anumite gazde); nu este o judecată asupra valorii temei.

| Temă | Indiciu | Motivul pentru care nu a primit specificație acum |
|------|---------|---------------------------------------------------|
| Randarea citărilor după stiluri CSL (procesor de stil) | README: lista de referințe e formatată simplu | Standardul public există (CSL 1.0.2) și a fost citit în parte, dar un procesor complet este de dimensiune XL; amânat pentru un lot propriu, cu un subset delimitat |
| Eliminarea antetelor și subsolurilor repetate din text PDF | README: PDF cu strat de text poppler | Nicio metodă publicată sau standard citit; formatul PDF (ISO 32000) nu definește antete/subsoluri ca structură |
| Detectare aproximativă de duplicate pe titlu (similaritate Jaccard / shingling) | cod: bibliografie din mai multe fișiere | Lucrările de referință (Jaccard, Broder 1997) nu au fost citite; spec 07 se limitează deliberat la identificatori |
| Diversificarea rezultatelor (Maximal Marginal Relevance) | cod: plafon pe document pentru a nu aglomera rezultatele | Articolul (Carbonell și Goldstein, SIGIR 1998) necitit; beneficiul pe biblioteci de teze nu e măsurat și există deja un plafon pe document |
| Metrici suplimentare pentru comanda de evaluare (nDCG, MAP) | README: eval are recall@k și MRR | Surse necitite (Järvelin și Kekäläinen 2002; Manning et al., cap. 8); valoare mică față de lipsa unui set real de întrebări (REVIEW.md) |
| Ponderi pe câmpuri la clasare (BM25F) | cod: se indexează doar textul pasajelor | Articolul BM25F necitit; indexul nu conține metadate de titlu/autor, deci ar cere întâi modificarea modelului de date |
| Limitarea ratei serverului local (token bucket, cod 429, antete de rată) | REVIEW.md: „nu au limitare a ratei” | RFC 9110 și RFC 6585 necitite; statutul antetelor de rată standardizate nu a fost verificat; candidat bun pentru lotul 2 |
| Parsarea fluxurilor de evenimente de la server pentru răspunsuri de model (SSE) și alte detalii ale API-urilor compatibile OpenAI | README: back-end-uri Ollama / Jan / OpenAI-compatibil | Standardul HTML (secțiunea despre server-sent events) și documentațiile publice ale serverelor locale necitite în sesiune; codul clientului de model nu a fost analizat |
| Numere de pagină pentru DOCX | README: DOCX fără pagini | Paginarea depinde de randare; nicio sursă (ISO/IEC 29500) citită; spec 11 acoperă doar EPUB, unde paginile sunt declarate în fișier |
| Marcare temporală de terță parte pentru chitanțe (RFC 3161) | README: chitanțe doar tamper-evident | Cere serviciu extern și are valoare juridică neclară; RFC necitit; exclus explicit din spec 14 |
| Gruparea cu cratime a ISBN | spec 02 | Cere fișierul de intervale al Agenției ISBN, a cărui licență nu a fost verificată |
| Contextul citărilor (cine a susținut sau contrazis o lucrare) | COMPETITION.md §3 | Cere un corp mare de enunțuri de citare clasificate; nu există standard sau metodă publică citită; dependent de date externe proprietare |
| Mod agent / scriere în gestionare externă de referințe | COMPETITION.md §3 | Interfețele sunt proprii fiecărui produs; nu s-a găsit standard public; ar însemna copierea comportamentului unui produs numit, ceea ce regulile kitului interzic |
| Extragere avansată de tabele și ecuații din PDF (modele de tip layout) | COMPETITION.md §3, README | Metode de cercetare cu modele și licențe proprii, fără standard; există deja o euristică în proiect |
| Metadate Dublin Core calificat și alte formate OAI (MARC, MODS) | spec 13 | Formatele nu au fost citite; spec 13 se oprește la Dublin Core necalificat, obligatoriu în protocol |
