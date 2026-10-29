const test = require('node:test'), assert = require('node:assert');
const core = require('./tezaur-core.js');

const idx = new core.Index();
idx.addDocument({ id: 'a.md', path: 'a.md', title: 'a', text: '# Burden\n\nThe burden of proof lies with the competent authority. Șarcina probei revine autorității.\n\n## Costs\n\nCompensation measures may be required.\n' });
idx.addDocument({ id: 'b.md', path: 'b.md', title: 'b', text: 'Completely unrelated text about gardening and tomatoes. '.repeat(10) });

test('chunking keeps offsets and sentence ends', () => {
  const t = 'One two three. '.repeat(120);
  const c = core.chunkText(t);
  assert.ok(c.length > 1);
  assert.ok(c.every(x => t.slice(x.offset).startsWith(x.text.slice(0, 10))));
  assert.ok(c[0].text.endsWith('.'));
});
test('search finds diacritic-folded text, headings and lines', () => {
  const h = idx.search('sarcina probei autoritate');
  assert.equal(idx.docs[idx.chunks[h[0].chunk].doc].id, 'a.md');
  assert.equal(idx.chunks[h[0].chunk].heading, 'Burden');
  assert.equal(idx.chunks[h[0].chunk].line, 0);
  assert.equal(h[0].cov, 1);
});
test('allow filter and removal', () => {
  assert.equal(idx.search('burden proof', 8, 3, d => d.id === 'b.md').length, 0);
  const i2 = new core.Index(); i2.addDocument({ id: 'x', path: 'x', title: 'x', text: 'burden of proof' }); i2.removeDocument('x');
  assert.equal(i2.search('burden').length, 0);
});
test('citation check flags invented numbers', () => {
  assert.deepEqual(core.checkAnswer(2, 'ok [1]. bad [5].').length, 1);
  assert.ok(core.checkAnswer(2, 'nothing cited').some(w => w.includes('no citations')));
});
test('answer() pipeline against both protocols', async () => {
  for (const api of ['ollama', 'openai']) {
    const seen = [];
    const http = async (url, o) => { seen.push(url); const body = JSON.parse(o.body);
      assert.equal(body.stream, false); assert.match(body.messages[1].content, /\[1\] \(a › Burden\)/);
      return { status: 200, text: JSON.stringify(api === 'ollama' ? { message: { content: 'The authority bears it [1]. Ghost [4].' } } : { choices: [{ message: { content: 'The authority bears it [1]. Ghost [4].' } }] }) }; };
    const r = await core.answer(idx, { api, model: 'm', host: 'http://h' }, http, 'ask', 'who bears the burden of proof',
      { labelFn: (d, c) => d.title + (c.heading ? ' › ' + c.heading : '') });
    assert.ok(seen[0].endsWith(api === 'ollama' ? '/api/chat' : '/chat/completions'));
    assert.ok(r.warnings.some(w => w.includes('non-existent')));
  }
});
test('server errors are explained', async () => {
  await assert.rejects(core.chat({ api: 'jan', model: 'm' }, async () => { throw new Error('ECONNREFUSED'); }, 's', 'u'), /Cannot reach http:\/\/127.0.0.1:1337\/v1/);
  await assert.rejects(core.chat({ api: 'jan', model: 'm' }, async () => ({ status: 500, text: 'boom' }), 's', 'u'), /500/);
  assert.deepEqual(await core.listModels({ api: 'ollama' }, async () => ({ status: 200, text: '{"models":[{"name":"llama3"}]}' })), ['llama3']);
});
