'use strict';
/** Client for `tezaur serve`. Node 18+ / browsers (global fetch). */
class TezaurError extends Error {}
class Client {
  constructor(url = 'http://127.0.0.1:8787', { token, workspace } = {}) { this.url = url.replace(/\/$/, ''); this.token = token; this.ws = workspace; }
  async _call(method, path, params = {}, body, raw) {
    const p = new URLSearchParams(params); if (this.ws) p.set('ws', this.ws);
    const headers = {}; if (this.token) headers.Authorization = 'Bearer ' + this.token;
    if (body !== undefined && !raw) headers['Content-Type'] = 'application/json';
    let r;
    try { r = await fetch(this.url + path + (p.toString() ? '?' + p : ''), { method, headers, body: body === undefined ? undefined : (raw ? body : JSON.stringify(body)) }); }
    catch (e) { throw new TezaurError('cannot reach ' + this.url); }
    const text = await r.text(); let d; try { d = JSON.parse(text); } catch { d = text; }
    if (!r.ok) throw new TezaurError(r.status + ': ' + (d && d.error || r.statusText));
    return d;
  }
  health() { return this._call('GET', '/api/health'); }
  sources() { return this._call('GET', '/api/sources'); }
  async search(q, k = 8) { return (await this._call('POST', '/api/search', {}, { q, k })).hits; }
  ask(q, mode = 'ask', k = 8, lang) { return this._call('POST', '/api/ask', {}, { q, mode, k, ...(lang ? { lang } : {}) }); }
  bib(fmt = 'csl') { return this._call('GET', '/api/bib', { fmt }); }
  mesh_search(q, k = 8) { return this._call('GET', '/api/search', { q, k, mesh: '1' }); }
  mesh_pull(peer) { return this._call('POST', '/api/mesh/pull', peer ? { peer } : {}, '', true); }
}
module.exports = { Client, TezaurError };
