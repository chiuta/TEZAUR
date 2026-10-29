# Recenzie adversarială — tezaur.chiuta.ro (29 oct. 2026)

Metodă: audit din GitHub Actions (site-ul live nu e accesibil din sandbox) — rularea 40955915164, joburile `http`, `browsers (chromium|firefox|webkit)` din `scripts/audit/*` — plus verificări locale pe aceeași sursă. Fiecare constatare are un filmuleț în `review-videos/` (ro + en). Rândurile marcate EVIDENȚĂ din filmulețe reproduc ce a tipărit CI-ul; comenzile fără marcaj au fost executate cu adevărat.

## Verdict
Site-ul funcționează: TLS valid (până 2027-01-27, TLSv1.3), HTTP→HTTPS 301, 0 erori în consolă, 0 cereri eșuate, 13 videoclipuri se încarcă și rulează în Chromium, Firefox și WebKit, fără depășire orizontală la 360 px, încărcare 0,2–0,5 s (Chromium, Fast-3G simulat). Dar am găsit defecte reale; aproape toate sunt reparate; rămân doar cele care nu depind de cod (vezi mai jos).

## Defecte confirmate
| # | Defect | Gravitate | Stare | Film |
|---|---|---|---|---|
| 1 | **Alarme false în auditul de citări**: „4 months” față de „four months” în sursă era marcat `number-mismatch` (v0.4.0 marca toate cele 3 citări, deși 2 erau corecte). | mare (încrederea în instrument) | REPARAT: cifrele se compară ca valoare (4 = four = patru), test de regresie; „5 years” rămâne marcat | R1 |
| 2 | **Safari/WebKit: subtitrări duble** — mp4-ul publicat avea 8 piste încorporate, iar pagina încărca și .vtt (două piste „fr”). Chromium/Firefox nu arată problema. | mediu | REPARAT: mp4-urile publice conțin doar video | R4 |
| 3 | **Fără antete de securitate** (CSP, X-Frame-Options, X-Content-Type-Options, Referrer-Policy, Permissions-Policy, HSTS) — pagina poate fi pusă în iframe. | mediu | PARȚIAL: GitHub Pages nu permite antete. Înlocuitori în pagină: CSP prin `<meta>` (script/style 'self'+inline, fără conexiuni externe; testat: 0 încălcări), `meta referrer`, protecție anti-iframe prin script (testată). Rămân imposibil de setat din pagină: X-Frame-Options, X-Content-Type-Options, Permissions-Policy, HSTS — cer proxy/CDN. | R2 |
| 4 | **`www.tezaur.chiuta.ro` nu se rezolvă** (NXDOMAIN). | mic | NEREPARABIL din cod: cere o înregistrare DNS `www CNAME chiuta.github.io.` la furnizorul domeniului; auditul îl raportează acum ca avertisment | R3 |
| 5 | **Fără canonical, Open Graph, favicon** — link distribuit = card gol, 404 pe /favicon.ico. | mic | REPARAT | R3 |
| 6 | **Contor static „01 / 16”** față de 20 de slide-uri (vizibil fără JS/înainte de JS). | mic | REPARAT | R5 |
| 7 | Butoanele de limbă (ro/en/…) nu aveau `aria-pressed`. | mic | REPARAT | — |
| 8 | Slide-ul cu videoclipuri era foarte lung pe mobil (13 stivuite, 3915 px la 360 px). | mic | REPARAT: pe ecrane înguste se văd primele 3 + buton „Arată toate”; 1529 px (măsurat în Chromium emulat, nu pe telefon real) | — |
| 9 | axe-core (WCAG 2.x A/AA + best-practice): aria-required-children pe #tdots, regiune derulabilă fără acces de la tastatură (#items, blocuri de cod, tabele), ținte tactile < 24 px (98 de noduri), landmark-uri lipsă, antet de tabel gol, contrast. | mare/mediu | REPARAT: 0 încălcări la 1280 px și 360 px (axe 4.x local; contrastul semnalat era tranzitoriu, în timpul animației de apariție). `scripts/audit/axe_check.py`. | — |

## Constatări din lista mea inițială pe care le retrag
* „Canvasul ignoră reduced-motion”: fals — codul oprește animația (`if(!reduce)requestAnimationFrame`).
* „Microfon = cloud nemenționat”: în PWA există un `confirm` care avertizează că Chrome/Edge trimit sunetul unui serviciu cloud; în deck nu am găsit o afirmație „100% local” care să îl contrazică.
* „~90 de moduri” din sursa HTML: JS îl înlocuiește la încărcare cu numărul real (CAT.length); doar HTML-ul brut e inexact.

## Limite ale auditului (nesigure / neverificate)
* Verificarea statică „0 slides” din `live_audit.py` era o eroare a scriptului meu (nu decomprima gzip), nu un defect al site-ului. Corectată; scriptul verifică acum și contorul, canonical/og/favicon.
* axe-core rulat doar local pe copia site-ului, nu pe domeniul live (verificarea live o fac browserele din CI, nu axe); un test automat nu înlocuiește un cititor de ecran.
* Nerulat: citirea cu cititor de ecran real, Safari iOS real, performanță pe 6 M de cuvinte (afirmația „~0,14 s”), starea link-urilor externe.
* Subtitrările în fr/it/es/pt/de/hu sunt traduse de un AI, fără revizuire nativă; filmulețele recenziei au doar ro + en.
