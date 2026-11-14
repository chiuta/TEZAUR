# TEZAUR — Un asistent de cercetare local, ancorat în citări, pentru scrierea unei teze. Un singur binar Rust.

[English](README.md) · **Română** · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versiune: 0.5.2 · Ultima actualizare: 2026-11-14

## Cum funcționează

Cărțile și articolele tale rămân pe calculatorul tău. TEZAUR le împarte în fragmente marcate cu pagina, le indexează (BM25 și, opțional, embeddings), regăsește pasajele pentru o întrebare sau un titlu de secțiune, iar un model local scrie numai pe baza acestor extrase, cu citări [n]. O verificare ulterioară semnalează citările inventate și construiește lista de referințe.

## Comenzi

- `tezaur search`: doar pasaje, fără model.
- `tezaur ask`: un răspuns cu citări; `compare`, `gaps`, `summarize` și `outline` lucrează pe aceleași extrase.

## Moduri de rulare

- Binarul Rust în linia de comandă, cu depozite de pachete construite de CI.
- Format PortableApps.com pentru Windows (`build-portable.bat`).
- Un singur fișier HTML offline, `tezaur.html`, cu indexul păstrat în browser.

## Limite sincere

- Nu este un scriitor de teze: redactează din sursele tale; argumentul și verificarea îți aparțin.
- Verificarea citărilor este mecanică: nu poate spune dacă un pasaj citat susține cu adevărat o propoziție, deci deschide pagina.
- Calitatea regăsirii este plafonul: dacă `search` nu scoate pasajul potrivit, `ask` nu îl poate folosi.

## Confidențialitate

Nimic nu iese de pe calculatorul tău decât dacă rulezi una dintre comenzile online opționale (vezi „Network use” în README-ul complet).

Documentație completă (în engleză): [README.md](README.md).

## Licență

MIT — Copyright (c) 2026 Alexandru-Ionuț Chiuță (Alexio).

## Sprijin

Trade-free: fără urmărire, fără reclame. Dacă îți este de folos, îl poți sprijini pe autor: https://buymeacoffee.com/echo.of.the.strings
