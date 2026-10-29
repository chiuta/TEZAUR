# usage: (cd $AXE_DIR && npm i axe-core); SITE_DIR=<dir with index.html> python3 scripts/audit/axe_check.py   -- prints WCAG violations at 1280 and 360 px
import sys, json, http.server, threading, functools
from playwright.sync_api import sync_playwright
S=__import__("os").environ.get("AXE_DIR","/tmp/axe")
root=__import__("os").environ.get("SITE_DIR",".")
h=functools.partial(http.server.SimpleHTTPRequestHandler,directory=root)
h.log_message=lambda *a:None
srv=http.server.ThreadingHTTPServer(("127.0.0.1",8897),h); threading.Thread(target=srv.serve_forever,daemon=True).start()
axe=open(S+"/node_modules/axe-core/axe.min.js").read()
with sync_playwright() as p:
    b=p.chromium.launch()
    for name,vp in (("desktop",{"width":1280,"height":800}),("mobile",{"width":360,"height":740})):
        pg=b.new_page(viewport=vp); errs=[]
        pg.on("console",lambda m:errs.append(m.text) if m.type=="error" else None); pg.on("pageerror",lambda e:errs.append(str(e)))
        pg.goto("http://127.0.0.1:8897/index.html"); pg.wait_for_timeout(2500); pg.evaluate("document.querySelectorAll('.rv').forEach(e=>e.classList.add('in'))"); pg.wait_for_timeout(1500)
        pg.evaluate(axe); r=pg.evaluate("axe.run(document,{runOnly:['wcag2a','wcag2aa','wcag21aa','wcag22aa','best-practice']})")
        print(name,"console errors:",errs, "height:",pg.evaluate("document.querySelector('#vid').offsetHeight"))
        for v in r["violations"]: print(" ",v["impact"],v["id"],len(v["nodes"]),v["nodes"][0]["target"][:1], v["nodes"][0]["any"][0]["message"][:160] if v["nodes"][0]["any"] else "")
