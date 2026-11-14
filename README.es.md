# TEZAUR — Un asistente de investigación local, basado en citas, para escribir una tesis. Un solo binario de Rust.

[English](README.md) · [Română](README.ro.md) · [Français](README.fr.md) · [Italiano](README.it.md) · **Español** · [Português](README.pt.md) · [Deutsch](README.de.md) · [Magyar](README.hu.md)

Versión: 0.5.2 · Última actualización: 2026-11-14

## Cómo funciona

Tus libros y artículos se quedan en tu equipo. TEZAUR los divide en fragmentos etiquetados con la página, los indexa (BM25 y, opcionalmente, embeddings), recupera los pasajes para una pregunta o un título de sección, y un modelo local escribe solo a partir de esos extractos, con citas [n]. Una comprobación posterior señala las citas inventadas y construye la lista de referencias.

## Comandos

- `tezaur search`: solo pasajes, sin modelo.
- `tezaur ask`: una respuesta con citas; `compare`, `gaps`, `summarize` y `outline` trabajan con los mismos extractos.

## Formas de ejecutarlo

- El binario de Rust en la línea de comandos, con repositorios de paquetes construidos por la CI.
- Formato PortableApps.com para Windows (`build-portable.bat`).
- Un solo archivo HTML sin conexión, `tezaur.html`, con el índice guardado en el navegador.

## Límites honestos

- No es un redactor de tesis: redacta a partir de tus fuentes; el argumento y la verificación son tuyos.
- La comprobación de citas es mecánica: no puede decir si un pasaje citado respalda de verdad una frase, así que abre la página.
- La calidad de la recuperación es el techo: si `search` no muestra el pasaje correcto, `ask` no puede usarlo.

## Privacidad

Nada sale de tu equipo salvo que ejecutes uno de los comandos en línea opcionales (consulta «Network use» en el README completo).

Documentación completa (en inglés): [README.md](README.md).

## Licencia

MIT — Copyright (c) 2026 Alexandru-Ionuț Chiuță (Alexio).

## Apoyo

Trade-free: sin rastreo, sin anuncios. Si te resulta útil, puedes apoyar al autor: https://buymeacoffee.com/echo.of.the.strings
