#!/usr/bin/env node
'use strict';
const { Client, TezaurError } = require('../index.js');
const [cmd, ...rest] = process.argv.slice(2);
const c = new Client(process.env.TEZAUR_SERVER || 'http://127.0.0.1:8787', { token: process.env.TEZAUR_TOKEN });
const q = rest.join(' ');
(async () => {
  if (cmd === 'sources') for (const d of (await c.sources()).docs) console.log(`${d.title}${d.cite ? ' — ' + d.cite : ''}  (${d.chunks} chunks)`);
  else if (cmd === 'search') for (const h of await c.search(q)) console.log(`[${h.n}] ${h.label}\n    ${h.text.slice(0, 300).replace(/\s+/g, ' ')}`);
  else if (['ask', 'compare', 'gaps'].includes(cmd)) {
    const r = await c.ask(q, cmd); console.log(r.answer);
    if (r.references) console.log('\n## References\n' + r.references);
    for (const w of r.warnings) console.error('⚠ ' + w);
  } else { console.error('usage: tezaur-js ask|compare|gaps|search|sources <text>   (TEZAUR_SERVER, TEZAUR_TOKEN)'); process.exit(2); }
})().catch(e => { console.error('tezaur-js: ' + (e instanceof TezaurError ? e.message : e)); process.exit(1); });
