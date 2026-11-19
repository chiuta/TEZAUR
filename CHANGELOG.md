# Jurnal de modificări — TEZAUR

## 0.6.0 — 2026-11-19

- Variantele HTML (`tezaur.html`, `index.html`, `pwa/index.html`) primesc stratul de compatibilitate cu browsere mai vechi (Chrome/Edge 55+, Firefox 52+, Safari 10.1+/iOS 10.3+): rezerve pentru API-urile care lipsesc în motoarele vechi și sintaxă JavaScript compatibilă ES2017. Fără modificări de funcționalitate intenționate. Verificat cu parser ES2017 și într-un motor vechi simulat, nu pe dispozitive reale.
- Corectat: numele cache-ului service worker-ului din `pwa/sw.js` se actualizează acum împreună cu pagina (`variants/build-pwa.py`), ca să primească modificarea și copiile deja instalate.
