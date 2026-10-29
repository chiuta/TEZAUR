"""Marimo notebook:  pip install marimo tezaur-client  &&  marimo edit marimo_app.py   (not run in the build sandbox)"""
import marimo

app = marimo.App()

@app.cell
def _():
    import os, marimo as mo
    from tezaur_client import Client
    c = Client(os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787"), os.environ.get("TEZAUR_TOKEN"))
    q = mo.ui.text(label="Question")
    q
    return c, mo, q

@app.cell
def _(c, mo, q):
    r = c.ask(q.value) if q.value else None
    mo.md(r["answer"] if r else "Type a question.")
    return

if __name__ == "__main__":
    app.run()
