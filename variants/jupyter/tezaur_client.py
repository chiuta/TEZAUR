"""Tiny Python client for `tezaur serve` (standard library only) — for notebooks and scripts.

    from tezaur_client import Tezaur
    t = Tezaur("http://127.0.0.1:8787", token="…", workspace="cap1")
    t.search("burden of proof")          # list of passages with citation labels
    t.ask("Who bears the burden?")       # answer with [n] citations + mechanical check
"""
import json, re, urllib.parse, urllib.request

SYSTEM = ("You are a careful academic writing assistant helping a researcher write a thesis. You are given numbered SOURCE "
          "EXCERPTS from the researcher's own library. Rules:\n1. Base every factual claim ONLY on the excerpts. Never use outside "
          "knowledge for facts, figures, dates or quotations.\n2. After each claim, cite the excerpt number(s) in square brackets, like "
          "[3] or [2, 5]. Never invent a number.\n3. If the excerpts do not contain enough to answer, say exactly what is missing "
          "instead of guessing.\n4. If excerpts disagree, present both positions and cite each.\n5. Keep quotations short and exact; "
          "otherwise paraphrase in scholarly prose.\n6. Write in the same language as the TASK unless told otherwise.")


class Tezaur:
    def __init__(self, url="http://127.0.0.1:8787", token=None, workspace="default", model=None, api=None):
        self.url, self.token, self.ws = url.rstrip("/"), token, workspace
        h = self._get("/api/health")
        self.model, self.api = model or h.get("model"), api or h.get("api")

    def _req(self, method, path, body=None, raw=False):
        sep = "&" if "?" in path else "?"
        ws = "" if path.startswith("/llm") else f"{sep}ws={urllib.parse.quote(self.ws)}"  # the model proxy needs no workspace
        r = urllib.request.Request(f"{self.url}{path}{ws}", method=method, data=body)
        if self.token:
            r.add_header("Authorization", "Bearer " + self.token)
        if body is not None and not raw:
            r.add_header("Content-Type", "application/json")
        with urllib.request.urlopen(r, timeout=600) as resp:
            return json.load(resp)

    def _get(self, path):
        return self._req("GET", path)

    def sources(self):
        return self._get("/api/sources")["docs"]

    def search(self, query, k=8, only=None):
        q = f"/api/search?q={urllib.parse.quote(query)}&k={k}"
        if only:
            q += "&paths=" + urllib.parse.quote("\n".join(only))
        return self._get(q)["hits"]

    def add(self, path):
        """Upload a file (pdf, docx, epub, txt, md…); the server reads it, with OCR for scans."""
        name = path.replace("\\", "/").split("/")[-1]
        with open(path, "rb") as f:
            return self._req("POST", "/api/ingest?name=" + urllib.parse.quote(name), f.read(), raw=True)

    def ask(self, question, k=8, task=None, lang=None):
        hits = self.search(question, k)
        if not hits:
            return {"answer": "No matching passages.", "hits": [], "warnings": []}
        prompt = "SOURCE EXCERPTS:\n\n" + "".join(f"[{h['n']}] ({h['label']})\n{h['text']}\n\n" for h in hits)
        prompt += "TASK:\n" + (task or f"Answer the question below as precisely as the excerpts allow.\n\nQUESTION: {question}")
        if lang:
            prompt += f"\n\nWrite the answer in: {lang}."
        msgs = [{"role": "system", "content": SYSTEM}, {"role": "user", "content": prompt}]
        if self.api == "ollama":
            out = self._req("POST", "/llm/api/chat", json.dumps({"model": self.model, "messages": msgs, "stream": False}).encode())["message"]["content"]
        else:
            out = self._req("POST", "/llm/chat/completions", json.dumps({"model": self.model, "messages": msgs, "stream": False}).encode())["choices"][0]["message"]["content"]
        cited = {int(x) for g in re.findall(r"\[(\d+(?:\s*[,;]\s*\d+)*)\]", out) for x in re.split(r"[,;]", g)}
        warn = []
        if not cited:
            warn.append("no citations at all — do not rely on this text")
        bad = sorted(n for n in cited if n < 1 or n > len(hits))
        if bad:
            warn.append(f"model cited non-existent excerpt(s): {bad}")
        return {"answer": out, "hits": hits, "warnings": warn}
