# tezaur-client

Python client for a running `tezaur serve` (standard library only), a command line, and an Anki `.apkg` exporter.

```python
from tezaur_client import Client
t = Client("http://127.0.0.1:8787", token=None)
r = t.ask("Who bears the burden of proof?")     # {'answer', 'references', 'warnings', 'excerpts'}
t.search("burden of proof"); t.sources(); t.bib("ris")
t.mesh_search("recognition")                      # also asks your peers
```

```bash
tezaur-py ask "Who bears the burden of proof?"      # TEZAUR_SERVER / TEZAUR_TOKEN
pip install "tezaur-client[anki]"
tezaur-apkg cards.tsv -o cards.apkg --deck "Thesis ch. 2"   # from `tezaur flashcards ... -o cards.tsv`
```
The `.apkg` was checked by reading its database back; it has not been imported into Anki itself.
