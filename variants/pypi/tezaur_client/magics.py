"""IPython / Jupyter magics for TEZAUR — JupyterLab, Jupyter Notebook, VS Code notebooks, Colab, Kaggle, Marimo-via-IPython.

    %load_ext tezaur_client.magics
    %tezaur_connect http://127.0.0.1:8787 [TOKEN]      # or set TEZAUR_SERVER / TEZAUR_TOKEN
    %tezaur_ask Who bears the burden of proof?
    %tezaur_compare How is X defined?        %tezaur_gaps recognition of qualifications
    %tezaur_search burden of proof
    %%tezaur_audit                           # the cell is a Markdown draft with [@key, p. 3] citations
    %%tezaur_coverage                        # the cell is an outline
    %%tezaur_novelty                         # the cell is text; what is new relative to your library

Leave the trailing "?" off a line magic (IPython reads it as help); the cell form `%%tezaur_ask` takes any text.
Every answer is returned as an object (`_` or assign it) with `.answer`, `.excerpts`, `.receipt`; the display is HTML-escaped.
Colab / Kaggle run in the cloud: they can only reach a TEZAUR server that is publicly reachable (e.g. through a tunnel you set up).
"""
import html
import json
import os

from . import Client, TezaurError

_client = None


def client():
    global _client
    if _client is None:
        _client = Client(os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787"), os.environ.get("TEZAUR_TOKEN"))
    return _client


def _h(s):
    return html.escape(str(s)).replace("\n", "<br>")


class Result(dict):
    """A dict with attribute access, rendered as HTML in notebooks."""
    def __getattr__(self, name):
        if name.startswith("_"):
            raise AttributeError(name)
        return self.get(name)

    def __init__(self, data, kind):
        super().__init__(data)
        self._kind = kind

    def _repr_html_(self):
        k = self._kind
        if k == "ask":
            out = ["<div class='tezaur'><div>%s</div>" % _h(self.get("answer", ""))]
            if self.get("references"):
                out.append("<details><summary>References</summary><pre>%s</pre></details>" % html.escape(self["references"]))
            for w in self.get("warnings", []):
                out.append("<div style='color:#b45309'>⚠ %s</div>" % _h(w))
            out.append("<details><summary>%d excerpts</summary>" % len(self.get("excerpts", [])))
            for e in self.get("excerpts", []):
                out.append("<p><b>[%s] %s</b><br>%s</p>" % (e["n"], _h(e["label"]), _h(e["text"])))
            out.append("</details></div>")
            return "".join(out)
        if k == "search":
            return "<div class='tezaur'>" + "".join("<p><b>[%s] %s</b> <small>(%d%% of terms)</small><br>%s</p>" % (h["n"], _h(h["label"]), round(h.get("cov", 0) * 100), _h(h["text"])) for h in self.get("hits", [])) + "</div>"
        if k == "audit":
            rows = "".join("<tr><td>%s</td><td>%s</td><td>%s</td></tr>" % (_h(f["verdict"]), _h(f["note"]), _h(f["sentence"])) for f in self.get("findings", []) if f["verdict"] not in ("supported", "quote-verbatim"))
            return "<div class='tezaur'><b>%d problem(s)</b> in %d checks. Mechanical check — open the page.<table>%s</table></div>" % (self.get("problems", 0), len(self.get("findings", [])), rows)
        if k == "coverage":
            rows = "".join("<tr><td>%s</td><td>%s</td><td>%s</td></tr>" % (_h(r["heading"]), _h(r["status"]), _h("; ".join(d["doc"] for d in r["docs"][:4]))) for r in self.get("rows", []))
            return "<div class='tezaur'><table><tr><th>Section</th><th>Status</th><th>Sources</th></tr>%s</table></div>" % rows
        if k == "novelty":
            return "<div class='tezaur'>%d covered, %d partly, <b>%d new</b> passages.</div>" % (self.get("covered", 0), self.get("partial", 0), self.get("new", 0))
        return "<pre>%s</pre>" % html.escape(json.dumps(dict(self), indent=1))

    def __repr__(self):
        return self.get("markdown") or self.get("answer") or json.dumps(dict(self), ensure_ascii=False)[:2000]


def _split(line):
    return line.strip()


def load_ipython_extension(ip):
    from IPython.core.magic import register_cell_magic, register_line_magic

    def guard(fn):
        def run(*a):
            try:
                return fn(*a)
            except TezaurError as e:
                print("TEZAUR: %s" % e)
        run.__name__ = fn.__name__
        run.__doc__ = fn.__doc__
        return run

    @register_line_magic
    @guard
    def tezaur_connect(line):
        """%tezaur_connect URL [TOKEN]"""
        global _client
        parts = line.split()
        if not parts:
            return Result(client().health(), "raw")
        _client = Client(parts[0], parts[1] if len(parts) > 1 else os.environ.get("TEZAUR_TOKEN"))
        return Result(_client.health(), "raw")

    def ask_like(mode):
        @guard
        def f(line):
            return Result(client().ask(_split(line), mode=mode), "ask")
        f.__doc__ = "%%tezaur_%s QUESTION — cited answer from your own library" % mode
        return f

    def ask_cell(mode):
        @guard
        def f(line, cell):
            return Result(client().ask(cell.strip(), mode=mode), "ask")
        return f

    for m in ("ask", "compare", "gaps"):
        register_line_magic("tezaur_" + m)(ask_like(m))
        register_cell_magic("tezaur_" + m)(ask_cell(m))

    @register_line_magic
    @guard
    def tezaur_search(line):
        """%tezaur_search QUERY — passages, no model"""
        return Result({"hits": client().search(_split(line))}, "search")

    @register_cell_magic
    @guard
    def tezaur_audit(line, cell):
        """%%tezaur_audit — check [@key, p. N] citations in the cell against the library"""
        return Result(client().audit(cell), "audit")

    @register_cell_magic
    @guard
    def tezaur_coverage(line, cell):
        """%%tezaur_coverage — outline in the cell; which sections have sources"""
        return Result(client().coverage(cell), "coverage")

    @register_cell_magic
    @guard
    def tezaur_novelty(line, cell):
        """%%tezaur_novelty — what the text in the cell adds beyond the library"""
        return Result(client().novelty(cell), "novelty")


def unload_ipython_extension(ip):
    pass
