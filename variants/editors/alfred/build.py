#!/usr/bin/env python3
"""Builds Tezaur.alfredworkflow (a zip): keyword `tz` -> script filter -> copy to clipboard. UNTESTED inside Alfred itself."""
import plistlib
import sys
import zipfile
from pathlib import Path

here = Path(__file__).parent
SF, CL = "11111111-1111-1111-1111-111111111111", "22222222-2222-2222-2222-222222222222"
plist = {
    "bundleid": "org.tezaur.alfred", "name": "TEZAUR", "description": "Search your research library", "createdby": "TEZAUR",
    "connections": {SF: [{"destinationuid": CL, "modifiers": 0, "modifiersubtext": "", "vitoclose": False}]},
    "objects": [
        {"uid": SF, "type": "alfred.workflow.input.scriptfilter", "version": 3, "config": {
            "keyword": "tz", "title": "Search the library", "withspace": True, "argumenttype": 1, "type": 0, "scriptargtype": 1,
            "script": "python3 tezaur_filter.py \"$1\"", "runningsubtext": "searching…", "queuemode": 1, "queuedelaymode": 0,
            "queuedelayimmediatelyinitially": False, "queuedelaycustom": 3, "alfredfiltersresults": False, "escaping": 102,
            "scriptfile": "", "subtext": "", "argumenttrimmode": 0, "argumentrequired": False, "argumentrequired_": False}},
        {"uid": CL, "type": "alfred.workflow.output.clipboard", "version": 3, "config": {"clipboardtext": "{query}", "autopaste": False, "transient": False}},
    ],
    "uidata": {SF: {"xpos": 50, "ypos": 50}, CL: {"xpos": 300, "ypos": 50}},
    "version": "0.1.0", "webaddress": "",
}
out = Path(sys.argv[1] if len(sys.argv) > 1 else "Tezaur.alfredworkflow")
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("info.plist", plistlib.dumps(plist))
    z.write(here / "tezaur_filter.py", "tezaur_filter.py")
print("wrote", out)
