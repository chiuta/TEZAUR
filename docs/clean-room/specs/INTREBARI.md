# Întrebări deschise din sesiunea de implementare (lotul 1)

Format: spec · punct · ce lipsește · ce s-a făcut.

1. **02 · ISBN-10** · Specificația marchează metoda ISBN-10 (modulo 11, ponderi 10…1) și conversia în ISBN-13 ca [dedus din cunoștințe generale] / [necunoscut]; sursa nu a fost citită. Testul 8 depinde de ea. S-a implementat exact formula din text (verificabilă din spec: `0-306-40615-2` → `9780306406157`), dar rămâne de confirmat din ISBN Users' Manual.
2. **01 · Lista tipurilor CSL (Appendix III)** · Lista oficială completă nu e reprodusă în spec; testul-proprietate 6 s-a rulat pe lista tipurilor menționate explicit în spec, nu pe lista oficială completă. Același lucru pentru lista RIS TY (lista din sursa secundară, doar codurile menționate în spec).
3. **01 · Majuscule la tipuri** · Spec e ambiguă (BibTeX: fără diferență de majuscule [dedus]; „majuscule neobișnuite → necunoscut”). S-a ales: BibTeX insensibil la majuscule; CSL și RIS exacte (RIS `jour` = necunoscut → GEN).
4. **06 · Enumerarea tipurilor CSL (45 de valori)** · Spec 06 nu o reproduce; fără ea nu se poate deosebi un tip CSL valid, dar neprevăzut în spec (ex. un tip rar), de unul inventat. Se tratează ca „necunoscut” (tip generic + avertisment) orice tip în afara celor menționate explicit în specificațiile 01 și 06. Testul 8 (`inventat`) trece; pierderea e posibilă pentru tipuri valide neenumerate. Ce e nevoie: lista oficială Appendix III / schema.
5. **03 · Câmpuri obligatorii pe tip** · Spec 03 dă ca exemplu doar `article`; avertismentul „lipsă câmp obligatoriu” e implementat numai pentru `article` (autor, titlu, jurnal, an). Celelalte tipuri: lista din manual nu e reprodusă în spec.
6. **06 · Descoperirea fișierelor CSL JSON** · Spec nu spune ce fișiere `.json` se încarcă automat. S-a ales: automat doar `*.csl.json` din directoarele de date; orice `.json` dat explicit prin `--bib`.
7. **07 · Test de acceptanță 2 (ISBN)** · Textul dă `978-92-95055-12-4` și `9789295505124`; cifrele diferă (…055… vs …505…), deci literal cele două nu sunt același ISBN. S-a testat intenția (același număr cu și fără cratime: `9789295055124`). De confirmat de titular.
8. **07 · Pragul de conflict pe titlu** · Spec: „mai puțin de jumătate din jetoanele comune”, pragul [necunoscut]. S-a ales: jetoane comune / (setul mai mare) < 0,5.
