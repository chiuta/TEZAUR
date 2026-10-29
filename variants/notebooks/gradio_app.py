"""Gradio front-end (also what Hugging Face Spaces runs):  pip install gradio tezaur-client  &&  python gradio_app.py
Not run in the build sandbox."""
import os
import gradio as gr
from tezaur_client import Client

c = Client(os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787"), os.environ.get("TEZAUR_TOKEN"))

def ask(q, mode):
    r = c.ask(q, mode=mode)
    return r["answer"] + "\n\n" + r["references"] + "\n".join("⚠ " + w for w in r["warnings"])

def audit(t):
    return c.audit(t)["markdown"]

with gr.Blocks(title="TEZAUR") as demo:
    with gr.Tab("Ask"):
        gr.Interface(ask, [gr.Textbox(label="Question"), gr.Radio(["ask", "compare", "gaps"], value="ask")], gr.Textbox(label="Answer"), allow_flagging="never")
    with gr.Tab("Audit"):
        gr.Interface(audit, gr.Textbox(lines=10, label="Draft"), gr.Markdown(), allow_flagging="never")

if __name__ == "__main__":
    demo.launch()
