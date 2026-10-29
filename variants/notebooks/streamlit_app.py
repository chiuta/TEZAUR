"""Streamlit front-end for a TEZAUR server:  pip install streamlit tezaur-client  &&  streamlit run streamlit_app.py
(Not run in the build sandbox; the client calls it makes are the same ones the tests cover.)"""
import os
import streamlit as st
from tezaur_client import Client

c = Client(os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787"), os.environ.get("TEZAUR_TOKEN"))
st.title("TEZAUR")
tab_ask, tab_audit, tab_cov = st.tabs(["Ask", "Audit a draft", "Outline coverage"])
with tab_ask:
    q = st.text_input("Question")
    mode = st.radio("Mode", ["ask", "compare", "gaps"], horizontal=True)
    if q:
        r = c.ask(q, mode=mode)
        st.write(r["answer"])
        for w in r["warnings"]:
            st.warning(w)
        with st.expander("Excerpts"):
            for e in r["excerpts"]:
                st.markdown("**[%d] %s**\n\n%s" % (e["n"], e["label"], e["text"]))
with tab_audit:
    t = st.text_area("Markdown draft with [@key, p. 3] citations", height=240)
    if t:
        for f in c.audit(t)["findings"]:
            if f["verdict"] not in ("supported", "quote-verbatim"):
                st.error("%s — %s\n\n> %s" % (f["verdict"], f["note"], f["sentence"]))
with tab_cov:
    o = st.text_area("Outline", height=160)
    if o:
        st.table([{"Section": r["heading"], "Status": r["status"], "Sources": r["sources"]} for r in c.coverage(o)["rows"]])
