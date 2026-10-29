# tezaur-client

```js
const { Client } = require('tezaur-client');
const t = new Client('http://127.0.0.1:8787', { token: process.env.TEZAUR_TOKEN });
const r = await t.ask('Who bears the burden of proof?');   // { answer, references, warnings, excerpts }
```
```bash
npx tezaur-client ask "Who bears the burden of proof?"   # or: npm i -g tezaur-client && tezaur-js ask ...
```
Needs a running `tezaur serve` (the Rust program). Answers come from your local model and can take minutes.
