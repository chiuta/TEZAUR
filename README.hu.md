# TEZAUR — Helyi, idézetalapú kutatási asszisztens szakdolgozat vagy disszertáció íráshoz. Egyetlen Rust-bináris.

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · **Magyar**

Verzió: 0.5.2 · Utolsó frissítés: 2026-11-14

## Hogyan működik

A könyveid és cikkeid a te gépeden maradnak. A TEZAUR oldalszámmal jelölt darabokra bontja őket, indexeli (BM25 és opcionálisan beágyazások), megkeresi a kérdéshez vagy szakaszcímhez tartozó részleteket, a helyi modell pedig csak ezekből a kivonatokból ír, [n] idézetekkel. Az utólagos ellenőrzés jelzi a kitalált idézeteket, és összeállítja a hivatkozásjegyzéket.

## Parancsok

- `tezaur search`: csak részletek, modell nélkül.
- `tezaur ask`: idézetekkel ellátott válasz; a `compare`, `gaps`, `summarize` és `outline` ugyanezekből a kivonatokból dolgozik.

## Futtatási módok

- A Rust parancssori bináris, a CI által épített csomagtárakkal.
- PortableApps.com formátum Windowshoz (`build-portable.bat`).
- Egyetlen offline HTML-fájl, `tezaur.html`, az indexet a böngészőben tárolva.

## Őszinte korlátok

- Nem szakdolgozatíró: a forrásaidból fogalmaz; az érvelés és az ellenőrzés a tiéd.
- Az idézetellenőrzés mechanikus: nem tudja megmondani, hogy egy idézett részlet valóban alátámaszt-e egy mondatot, ezért nyisd meg az oldalt.
- A keresés minősége a plafon: ha a `search` nem hozza elő a megfelelő részletet, az `ask` nem tudja felhasználni.

## Adatvédelem

Semmi sem hagyja el a gépedet, hacsak nem futtatod valamelyik opcionális online parancsot (lásd a teljes README „Network use” részét).

Teljes dokumentáció (angolul): [README.md](README.md).

## Licenc

MIT — Copyright (c) 2026 Alexandru-Ionuț Chiuță (Alexio).

## Támogatás

Trade-free: nincs követés, nincs reklám. Ha hasznos, támogathatod a szerzőt: https://buymeacoffee.com/echo.of.the.strings
