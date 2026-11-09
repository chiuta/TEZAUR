# Întrebări deschise din sesiunea de implementare (lotul 1)

Format: spec · punct · ce lipsește · ce s-a făcut.

1. **02 · ISBN-10** · Specificația marchează metoda ISBN-10 (modulo 11, ponderi 10…1) și conversia în ISBN-13 ca [dedus din cunoștințe generale] / [necunoscut]; sursa nu a fost citită. Testul 8 depinde de ea. S-a implementat exact formula din text (verificabilă din spec: `0-306-40615-2` → `9780306406157`), dar rămâne de confirmat din ISBN Users' Manual.
2. **01 · Lista tipurilor CSL (Appendix III)** · Lista oficială completă nu e reprodusă în spec; testul-proprietate 6 s-a rulat pe lista tipurilor menționate explicit în spec, nu pe lista oficială completă. Același lucru pentru lista RIS TY (lista din sursa secundară, doar codurile menționate în spec).
3. **01 · Majuscule la tipuri** · Spec e ambiguă (BibTeX: fără diferență de majuscule [dedus]; „majuscule neobișnuite → necunoscut”). S-a ales: BibTeX insensibil la majuscule; CSL și RIS exacte (RIS `jour` = necunoscut → GEN).
