# TEZAUR — Un assistente di ricerca locale, fondato sulle citazioni, per scrivere una tesi. Un solo binario Rust.

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · **Italiano** · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versione: 0.5.2 · Ultimo aggiornamento: 2026-11-14

## Come funziona

I tuoi libri e articoli restano sul tuo computer. TEZAUR li divide in frammenti etichettati con la pagina, li indicizza (BM25 e, facoltativamente, embedding), recupera i passaggi per una domanda o un titolo di sezione, e un modello locale scrive solo da quegli estratti, con citazioni [n]. Una verifica finale segnala le citazioni inventate e costruisce l’elenco dei riferimenti.

## Comandi

- `tezaur search`: solo passaggi, senza modello.
- `tezaur ask`: una risposta con citazioni; `compare`, `gaps`, `summarize` e `outline` lavorano sugli stessi estratti.

## Modi di eseguirlo

- Il binario Rust da riga di comando, con repository di pacchetti costruiti dalla CI.
- Formato PortableApps.com per Windows (`build-portable.bat`).
- Un solo file HTML offline, `tezaur.html`, con l’indice conservato nel browser.

## Limiti dichiarati

- Non è uno scrittore di tesi: redige a partire dalle tue fonti; l’argomentazione e la verifica restano tue.
- La verifica delle citazioni è meccanica: non può dire se un passaggio citato sostiene davvero una frase, quindi apri la pagina.
- La qualità del recupero è il tetto: se `search` non fa emergere il passaggio giusto, `ask` non può usarlo.

## Riservatezza

Nulla lascia il tuo computer, a meno che tu non esegua uno dei comandi online facoltativi (vedi «Network use» nel README completo).

Documentazione completa (in inglese): [README.md](README.md).

## Licenza

MIT — Copyright (c) 2026 Alexandru-Ionuț Chiuță (Alexio).

## Sostegno

Trade-free: niente tracciamento, niente pubblicità. Se ti è utile, puoi sostenere l’autore: https://buymeacoffee.com/echo.of.the.strings
