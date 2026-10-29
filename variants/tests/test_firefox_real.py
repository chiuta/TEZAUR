#!/usr/bin/env python3
"""Real Firefox (selenium + geckodriver): the Firefox build of the extension installs and its app page runs.
Needs dist/tezaur-firefox.zip (variants/build-extension.sh). Run in CI (real-variants.yml)."""
import json, os, sys, time
from selenium import webdriver
from selenium.webdriver.firefox.options import Options
from selenium.webdriver.common.by import By
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
UUID = "5b0e2b2a-4c0f-4a53-9f10-0c6e6c1f0001"
o = Options(); o.add_argument("-headless")
o.set_preference("extensions.webextensions.uuids", json.dumps({"tezaur@trom.tf": UUID}))
o.set_preference("xpinstall.signatures.required", False)
d = webdriver.Firefox(options=o); ok = True
try:
    aid = d.install_addon(os.path.join(ROOT, "dist", "tezaur-firefox.zip"), temporary=True)
    print("installed add-on id:", aid); ok &= aid == "tezaur@trom.tf"
    time.sleep(2)
    url = "moz-extension://%s/app/index.html" % UUID
    d.set_context("chrome")  # marionette cannot navigate to moz-extension: URLs; the browser UI can open them
    d.execute_script("gBrowser.selectedTab = gBrowser.addTab(arguments[0], {triggeringPrincipal: Services.scriptSecurityManager.getSystemPrincipal()});", url)
    d.set_context("content"); time.sleep(3)
    d.switch_to.window(d.window_handles[-1]); time.sleep(2)
    title = d.title; body = d.find_element(By.TAG_NAME, "body").text
    print("title:", title, "| body chars:", len(body))
    ok &= "TEZAUR" in title and len(body) > 20
    errs = d.execute_script("return window.__errs||[]"); print("ui elements:", len(d.find_elements(By.CSS_SELECTOR, "textarea,input,button")))
    ok &= len(d.find_elements(By.CSS_SELECTOR, "textarea,input,button")) > 2
finally:
    d.quit()
print("PASS" if ok else "FAIL"); sys.exit(0 if ok else 1)
