# TEZAUR — Ein lokaler, zitatbasierter Recherche-Assistent zum Schreiben einer Abschlussarbeit. Eine einzige Rust-Binärdatei.

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · **Deutsch** · [Magyar](README.hu.md)

Version: 0.5.2 · Letzte Aktualisierung: 2026-11-14

## So funktioniert es

Deine Bücher und Artikel bleiben auf deinem Rechner. TEZAUR zerlegt sie in seitenmarkierte Abschnitte, indexiert sie (BM25 und optional Embeddings), findet die Passagen zu einer Frage oder einem Abschnittstitel, und ein lokales Modell schreibt nur aus diesen Auszügen, mit Zitaten [n]. Eine Nachprüfung markiert erfundene Zitate und erstellt das Literaturverzeichnis.

## Befehle

- `tezaur search`: nur Passagen, ohne Modell.
- `tezaur ask`: eine Antwort mit Zitaten; `compare`, `gaps`, `summarize` und `outline` arbeiten mit denselben Auszügen.

## Möglichkeiten, es auszuführen

- Die Rust-Binärdatei auf der Kommandozeile, mit von der CI gebauten Paketquellen.
- PortableApps.com-Format für Windows (`build-portable.bat`).
- Eine einzelne Offline-HTML-Datei, `tezaur.html`, mit dem Index im Browser.

## Ehrliche Grenzen

- Es ist kein Abschlussarbeit-Schreiber: Es formuliert aus deinen Quellen; Argumentation und Überprüfung bleiben bei dir.
- Die Zitatprüfung ist mechanisch: Sie kann nicht sagen, ob eine zitierte Passage einen Satz wirklich stützt – also öffne die Seite.
- Die Qualität der Suche ist die Obergrenze: Findet `search` die richtige Passage nicht, kann `ask` sie nicht nutzen.

## Datenschutz

Nichts verlässt deinen Rechner, außer du führst einen der optionalen Online-Befehle aus (siehe „Network use“ in der vollständigen README).

Vollständige Dokumentation (Englisch): [README.md](README.md).

## Lizenz

MIT — Copyright (c) 2026 Alexandru-Ionuț Chiuță (Alexio).

## Unterstützung

Trade-free: kein Tracking, keine Werbung. Wenn es dir nützt, kannst du den Autor unterstützen: https://buymeacoffee.com/echo.of.the.strings
