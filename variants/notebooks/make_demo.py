"""Generates tezaur-demo.ipynb (and is reused by the test). Run: python3 make_demo.py"""
import nbformat as nbf

def build():
    nb = nbf.v4.new_notebook()
    md, code = nbf.v4.new_markdown_cell, nbf.v4.new_code_cell
    nb.cells = [
        md("# TEZAUR in a notebook\nStart `tezaur serve` first (or set `TEZAUR_SERVER`). Every answer cites passages of *your* library."),
        code("%load_ext tezaur_client.magics\n%tezaur_connect"),
        code("%tezaur_search burden of proof"),
        code("r = %tezaur_ask Who bears the burden of proof\nr"),
        code("%%tezaur_audit\nThe burden of proof lies with the body assessing the qualification [@lisbon_convention].\nPenguins sing opera in Antarctica every single winter morning loudly [@lisbon_convention]."),
        code("%%tezaur_coverage\n# Recognition of qualifications\n# Quantum gravity"),
        code("print(r.receipt['digest'])\nfrom tezaur_client.magics import client\nclient().verify_receipt(r.receipt, r.answer)['ok']"),
    ]
    nb.metadata["kernelspec"] = {"name": "python3", "display_name": "Python 3", "language": "python"}
    return nb

if __name__ == "__main__":
    nbf.write(build(), "tezaur-demo.ipynb")
