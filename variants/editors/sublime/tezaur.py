"""TEZAUR for Sublime Text 4: Command Palette -> "TEZAUR: Ask" (uses the selection as the question when there is one).
Settings: Preferences > Package Settings > TEZAUR (tezaur.sublime-settings): server, token, k."""
import json
import threading
import urllib.request

import sublime
import sublime_plugin


def format_result(mode, d):
    if "error" in d:
        return "Error: " + d["error"]
    if mode == "search":
        return "\n".join("[%d] %s\n    %s" % (h["n"], h["label"], " ".join(h["text"][:300].split())) for h in d.get("hits", [])) or "No matching passages."
    out = d["answer"]
    if d.get("references"):
        out += "\n\n## References\n" + d["references"]
    for w in d.get("warnings", []):
        out += "\n⚠ " + w
    return out


def call(server, token, mode, q, k):
    path = "/api/search" if mode == "search" else "/api/ask"
    req = urllib.request.Request(server.rstrip("/") + path, data=json.dumps({"q": q, "mode": mode, "k": k}).encode(),
                                 headers={"Content-Type": "application/json"})
    if token:
        req.add_header("Authorization", "Bearer " + token)
    try:
        with urllib.request.urlopen(req, timeout=600) as r:
            return json.load(r)
    except Exception as e:  # noqa
        return {"error": "cannot reach %s (%s)" % (server, e)}


class _Base(sublime_plugin.TextCommand):
    mode = "ask"

    def run(self, edit, question=None):
        sel = self.view.sel()
        picked = " ".join(self.view.substr(r) for r in sel if not r.empty()).strip()
        if question or picked:
            self._go(question or picked)
        else:
            self.view.window().show_input_panel("TEZAUR %s:" % self.mode, "", self._go, None, None)

    def _go(self, q):
        s = sublime.load_settings("tezaur.sublime-settings")
        sublime.status_message("TEZAUR: thinking…")

        def work():
            d = call(s.get("server", "http://127.0.0.1:8787"), s.get("token"), self.mode, q, s.get("k", 6))
            text = format_result(self.mode, d)
            sublime.set_timeout(lambda: self._show(text), 0)
        threading.Thread(target=work, daemon=True).start()

    def _show(self, text):
        v = self.view.window().new_file()
        v.set_name("TEZAUR")
        v.set_scratch(True)
        v.assign_syntax("scope:text.html.markdown")
        v.run_command("append", {"characters": text})


class TezaurAskCommand(_Base):
    mode = "ask"


class TezaurCompareCommand(_Base):
    mode = "compare"


class TezaurGapsCommand(_Base):
    mode = "gaps"


class TezaurSearchCommand(_Base):
    mode = "search"
