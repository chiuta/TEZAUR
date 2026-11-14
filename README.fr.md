# TEZAUR — Un assistant de recherche local, fondé sur les citations, pour rédiger une thèse. Un seul binaire Rust.

[English](README.md) · [Română](README.ro.md) · **Français** · [Italiano](README.it.md) · [Español](README.es.md) · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Version : 0.5.2 · Dernière mise à jour : 2026-11-14

## Fonctionnement

Vos livres et articles restent sur votre machine. TEZAUR les découpe en fragments étiquetés par page, les indexe (BM25 et, en option, embeddings), retrouve les passages pour une question ou un titre de section, puis un modèle local rédige uniquement à partir de ces extraits, avec des citations [n]. Une vérification finale signale les citations inventées et construit la liste de références.

## Commandes

- `tezaur search` : uniquement des passages, sans modèle.
- `tezaur ask` : une réponse citée ; `compare`, `gaps`, `summarize` et `outline` travaillent sur les mêmes extraits.

## Façons de l’utiliser

- Le binaire Rust en ligne de commande, avec des dépôts de paquets construits par la CI.
- Format PortableApps.com pour Windows (`build-portable.bat`).
- Un seul fichier HTML hors ligne, `tezaur.html`, avec l’index conservé dans le navigateur.

## Limites assumées

- Ce n’est pas un rédacteur de thèse : il rédige à partir de vos sources ; l’argument et la vérification vous appartiennent.
- La vérification des citations est mécanique : elle ne peut pas dire si un passage cité soutient vraiment une phrase, alors ouvrez la page.
- La qualité de la recherche est le plafond : si `search` ne fait pas remonter le bon passage, `ask` ne peut pas l’utiliser.

## Confidentialité

Rien ne quitte votre machine, sauf si vous lancez l’une des commandes en ligne facultatives (voir « Network use » dans le README complet).

Documentation complète (en anglais) : [README.md](README.md).

## Licence

MIT — Copyright (c) 2026 Alexandru-Ionuț Chiuță (Alexio).

## Soutien

Trade-free : sans suivi, sans publicité. Si cela vous est utile, vous pouvez soutenir l’auteur : https://buymeacoffee.com/echo.of.the.strings
