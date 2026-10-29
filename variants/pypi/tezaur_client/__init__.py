"""Client for `tezaur serve` (standard library only)."""
import json
import os
import urllib.error
import urllib.parse
import urllib.request

__all__ = ["Client", "TezaurError"]


class TezaurError(RuntimeError):
    pass


class Client:
    def __init__(self, url=None, token=None, workspace=None, timeout=600):
        url = url or os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787")
        self.url, self.token, self.ws, self.timeout = url.rstrip("/"), token or os.environ.get("TEZAUR_TOKEN"), workspace, timeout

    def _call(self, method, path, params=None, body=None, raw=False):
        params = dict(params or {})
        if self.ws:
            params["ws"] = self.ws
        full = self.url + path + ("?" + urllib.parse.urlencode(params) if params else "")
        data = body if raw else (json.dumps(body).encode() if body is not None else None)
        req = urllib.request.Request(full, data=data, method=method)
        if body is not None and not raw:
            req.add_header("Content-Type", "application/json")
        if self.token:
            req.add_header("Authorization", "Bearer " + self.token)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as r:
                payload = r.read()
        except urllib.error.HTTPError as e:
            try:
                msg = json.loads(e.read()).get("error", e.reason)
            except Exception:
                msg = e.reason
            raise TezaurError("%s: %s" % (e.code, msg)) from None
        except urllib.error.URLError as e:
            raise TezaurError("cannot reach %s (%s)" % (self.url, e.reason)) from None
        try:
            return json.loads(payload)
        except ValueError:
            return payload.decode("utf-8", "replace")

    def health(self): return self._call("GET", "/api/health")
    def sources(self): return self._call("GET", "/api/sources")
    def search(self, q, k=8): return self._call("POST", "/api/search", body={"q": q, "k": k})["hits"]
    def ask(self, q, mode="ask", k=8, lang=None):
        b = {"q": q, "mode": mode, "k": k}
        if lang:
            b["lang"] = lang
        return self._call("POST", "/api/ask", body=b)
    def audit(self, text, semantic=False):
        """Check a Markdown draft with [@key, p. N] citations: supported / weak / unsupported / wrong-page / quotes.
        No model involved unless semantic=True (slow second opinion from the local model; it can be wrong)."""
        b = {"text": text}
        if semantic:
            b["semantic"] = "1"
        return self._call("POST", "/api/audit", body=b)
    def refs(self, text):
        """Cited keys found nowhere, .bib entries and library documents never cited."""
        return self._call("POST", "/api/refs", body={"text": text})
    def coverage(self, outline):
        """Which headings of an outline have sources in the library."""
        return self._call("POST", "/api/coverage", body={"text": outline})
    def novelty(self, text):
        """Which passages of a text are already covered by the library and which are new."""
        return self._call("POST", "/api/novelty", body={"text": text})
    def verify_receipt(self, receipt, answer=None):
        """Check a receipt returned by ask(): untouched, answer matches, excerpts still in the index."""
        b = {"receipt": receipt}
        if answer is not None:
            b["answer"] = answer
        return self._call("POST", "/api/receipt", body=b)
    def bib(self, fmt="csl"): return self._call("GET", "/api/bib", {"fmt": fmt})
    def read(self, path): return self._call("GET", "/api/read", {"path": path})
    def add(self, filename):
        """Upload a file (pdf, docx, epub, txt, md…); the server reads it, with OCR for scans."""
        with open(filename, "rb") as f:
            return self._call("POST", "/api/ingest", {"name": filename.replace("\\", "/").split("/")[-1]}, f.read(), raw=True)
    def remove(self, path): return self._call("DELETE", "/api/doc", {"path": path})

    # mesh
    def mesh_info(self): return self._call("GET", "/api/mesh/info")
    def mesh_search(self, q, k=8): return self._call("GET", "/api/search", {"q": q, "k": k, "mesh": "1"})
    def mesh_add_peer(self, name, url, token=None, proxy=None):
        return self._call("POST", "/api/mesh/peers", body={"name": name, "url": url, "token": token, "proxy": proxy})
    def mesh_pull(self, peer=None): return self._call("POST", "/api/mesh/pull", {"peer": peer} if peer else None, b"", raw=True)
