"""Gradio front-end (also what Hugging Face Spaces runs):  pip install gradio tezaur-client  &&  python gradio_app.py
Tested with Gradio 6 (gradio_client against the launched app)."""
import os
import gradio as gr
from tezaur_client import Client

c = Client(os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787"), os.environ.get("TEZAUR_TOKEN"))

def ask(q, mode):
    r = c.ask(q, mode=mode)
    return r["answer"] + "\n\n" + r["references"] + "\n".join("⚠ " + w for w in r["warnings"])

def audit(t):
    return c.audit(t)["markdown"]

with gr.Blocks(title="TEZAUR") as demo:  # plain Blocks + click(): works across Gradio 3 to 6 (Interface(allow_flagging=...) was removed in 5)
    with gr.Tab("Ask"):
        q = gr.Textbox(label="Question")
        mode = gr.Radio(["ask", "compare", "gaps"], value="ask", label="Mode")
        out = gr.Textbox(label="Answer")
        gr.Button("Ask").click(ask, [q, mode], out, api_name="ask")
    with gr.Tab("Audit"):
        t = gr.Textbox(lines=10, label="Draft")
        rep = gr.Markdown()
        gr.Button("Audit").click(audit, t, rep, api_name="audit")

if __name__ == "__main__":
    demo.launch()
