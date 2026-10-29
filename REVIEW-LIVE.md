# Recenzie adversarială — tezaur.chiuta.ro (29 oct. 2026)

Metodă: audit din GitHub Actions (site-ul live nu e accesibil din sandbox) — rularea 40955915164, joburile `http`, `browsers (chromium|firefox|webkit)` din `scripts/audit/*` — plus verificări locale pe aceeași sursă. Fiecare constatare are un filmuleț în `review-videos/` (ro + en). Rândurile marcate EVIDENȚĂ din filmulețe reproduc ce a tipărit CI-ul; comenzile fără marcaj au fost executate cu adevărat.

## Verdict
Site-ul funcționează: TLS valid (până 2027-01-27, TLSv1.3), HTTP→HTTPS 301, 0 erori în consolă, 0 cereri eșuate, 13 videoclipuri se încarcă și rulează în Chromium, Firefox și WebKit, fără depășire orizontală la 360 px, încărcare 0,2–0,5 s (Chromium, Fast-3G simulat). Dar am găsit defecte reale; șase sunt reparate în commit d02a465 și în commitul cu acest raport, trei rămân.

## Defecte confirmate
| # | Defect | Gravitate | Stare | Film |
|---|---|---|---|---|
| 1 | **Alarme false în auditul de citări**: „4 months” față de „four months” în sursă era marcat `number-mismatch` (v0.4.0 marca toate cele 3 citări, deși 2 erau corecte). | mare (încrederea în instrument) | REPARAT: cifrele se compară ca valoare (4 = four = patru), test de regresie; „5 years” rămâne marcat | R1 |
| 2 | **Safari/WebKit: subtitrări duble** — mp4-ul publicat avea 8 piste încorporate, iar pagina încărca și .vtt (două piste „fr”). Chromium/Firefox nu arată problema. | mediu | REPARAT: mp4-urile publice conțin doar video | R4 |
| 3 | **Fără antete de securitate** (CSP, X-Frame-Options, X-Content-Type-Options, Referrer-Policy, Permissions-Policy, HSTS) — pagina poate fi pusă în iframe. | mediu | NEREPARAT: GitHub Pages nu permite antete; adăugat doar `meta referrer`. Soluție reală: proxy/CDN în față. | R2 |
| 4 | **`www.tezaur.chiuta.ro` nu se rezolvă** (NXDOMAIN). | mic | NEREPARAT: cere o înregistrare DNS CNAME pe care o poți adăuga doar tu | R3 |
| 5 | **Fără canonical, Open Graph, favicon** — link distribuit = card gol, 404 pe /favicon.ico. | mic | REPARAT | R3 |
| 6 | **Contor static „01 / 16”** față de 20 de slide-uri (vizibil fără JS/înainte de JS). | mic | REPARAT | R5 |
| 7 | Butoanele de limbă (ro/en/…) nu aveau `aria-pressed`. | mic | REPARAT | — |
| 8 | Pagina de prezentare e foarte lungă pe mobil (13 videoclipuri stivuite, ~3900 px înălțime la 360 px). | mic | NEREPARAT (nu am măsurat pe dispozitiv real) | — |

## Constatări din lista mea inițială pe care le retrag
* „Canvasul ignoră reduced-motion”: fals — codul oprește animația (`if(!reduce)requestAnimationFrame`).
* „Microfon = cloud nemenționat”: în PWA există un `confirm` care avertizează că Chrome/Edge trimit sunetul unui serviciu cloud; în deck nu am găsit o afirmație „100% local” care să îl contrazică.
* „~90 de moduri” din sursa HTML: JS îl înlocuiește la încărcare cu numărul real (CAT.length); doar HTML-ul brut e inexact.

## Limite ale auditului (nesigure / neverificate)
* Verificarea statică „0 slides, no video section” din `live_audit.py` e o eroare de potrivire a expresiei din scriptul meu, nu un defect al site-ului (browserele numără 20 de slide-uri și contorul „01 / 20”). Scriptul trebuie corectat.
* axe-core (WCAG) rulat doar local pe o copie a site-ului, nu live: probleme semnalate (aria-required-children pe #tdots, regiune derulabilă fără acces de la tastatură pe #items, ținte tactile < 24 px, landmark-uri) NU au fost reparate.
* Nerulat: citirea cu cititor de ecran real, Safari iOS real, performanță pe 6 M de cuvinte (afirmația „~0,14 s”), starea link-urilor externe.
* Subtitrările în fr/it/es/pt/de/hu sunt traduse de un AI, fără revizuire nativă; filmulețele recenziei au doar ro + en.
