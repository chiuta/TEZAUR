# Antete de securitate pentru tezaur.chiuta.ro — variante gratuite

GitHub Pages nu permite antete proprii (X-Frame-Options, X-Content-Type-Options, Permissions-Policy, HSTS). Cele trei variante de mai jos le adaugă. Sunt ordonate după cât de bine se potrivesc cu filozofia trade-free a proiectului.

| # | Variantă | Cost | Cine o operează | Potrivire etică | Fișier |
|---|---|---|---|---|---|
| 1 | **Proxy propriu** (Caddy sau nginx) pe un server pe care îl administrezi deja (de ex. YunoHost) | 0 (ai serverul) | tu | maximă: fără terț, fără urmărire, poate rezolva și `www` | `Caddyfile`, `nginx.conf` |
| 2 | **Codeberg Pages** (Codeberg e.V., asociație non-profit germană, Forgejo, software liber) cu `_headers` | 0 | Codeberg e.V. | foarte bună: non-profit, fără publicitate | `_headers` |
| 3 | **Cloudflare** (Pages cu `_headers`, sau proxy în fața GitHub Pages) | 0 în planul gratuit | companie americană | compromis: ușor și rapid, dar centralizează traficul într-o singură firmă și nu e liber/trade-free | `_headers` |

## Ce știu sigur și ce nu
* Sintaxa `_headers` pentru Cloudflare Pages e din [documentația lor](https://developers.cloudflare.com/pages/configuration/headers/index.md) (limite: 100 de reguli, 2000 de caractere pe linie).
* Pentru Codeberg: [problema #2864](https://codeberg.org/Codeberg/Community/issues/2864) arată că `Content-Security-Policy` și `X-Frame-Options` au fost adăugate pe lista permisă la 2026-08-26. Lista completă a antetelor permise nu am văzut-o; verifică în manifestul git-pages după deploy ce antete refuză.
* **Nimic din acest director nu a fost rulat de mine** (nu am server, cont Codeberg sau Cloudflare la dispoziție): sintaxa Caddy/nginx e standard, dar netestată. Testează cu `curl -sI https://tezaur.chiuta.ro/` și cu `python3 scripts/audit/live_audit.py` (verificarea de antete trece dacă lipsesc doar cele nesetate).

## Note de aplicare
* **Varianta 1:** mută înregistrarea DNS A/AAAA a `tezaur.chiuta.ro` către serverul tău (și adaugă `www` ca CNAME/A către el). Lasă fișierul `CNAME` din repo, ca GitHub Pages să accepte cererile proxy-ate. Atenție: tot traficul trece prin serverul tău; dacă pică, pică site-ul.
* **HSTS:** `max-age=31536000` blochează accesul pe HTTP un an. Activează-l doar când ești sigur că HTTPS merge pe domeniu și `www`.
* **CSP:** se aplică doar pe `/` (prezentarea). PWA-ul din `/app/` încarcă alte resurse (Whisper, conexiuni la serverul local), deci primește doar antetele fără CSP.
* **Fără Cloudflare proxy?** E perfect rezonabil: variantele 1 și 2 acoperă aceleași antete fără să depinzi de ei.
