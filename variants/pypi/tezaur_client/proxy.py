"""jupyter-server-proxy entry: puts a TEZAUR tile in the JupyterLab launcher (`pip install tezaur-client[jupyter]`).
It starts `tezaur serve` on a free port inside your Jupyter session, using your TEZAUR_INDEX / tezaur.conf."""
import os
import shutil


def setup_tezaur():
    exe = os.environ.get("TEZAUR_BIN") or shutil.which("tezaur") or "tezaur"
    return {
        "command": [exe, "serve", "--port", "{port}"],
        "timeout": 30,
        "absolute_url": False,
        "launcher_entry": {"title": "TEZAUR", "enabled": True},
    }
