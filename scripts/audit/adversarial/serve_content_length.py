#!/usr/bin/env python3
"""Regression for the 0.5.0 crash: one request announcing a huge Content-Length must not kill `tezaur serve`.
usage: serve_content_length.py PATH_TO_TEZAUR WORKDIR   (WORKDIR must be empty or contain only a library)"""
import os, socket, subprocess, sys, time
T, W, PORT = sys.argv[1], sys.argv[2], 18390
os.makedirs(W, exist_ok=True)
srv = subprocess.Popen([T, "--index", os.path.join(W, "i.idx"), "serve", "--port", str(PORT)], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1.5)
def req(raw):
    s = socket.create_connection(("127.0.0.1", PORT), timeout=5); s.sendall(raw)
    try: return s.recv(300).split(b"\r\n")[0].decode()
    except Exception as e: return type(e).__name__
ok = True
for cl in (99999999999, 4000000000, 2**63 - 1, 2**64 - 1):
    r = req(f"POST /api/search HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nContent-Length: {cl}\r\nConnection: close\r\n\r\n{{}}".encode())
    alive = req(f"GET /api/health HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nConnection: close\r\n\r\n".encode()).endswith("200 OK") and srv.poll() is None
    print(f"Content-Length {cl}: {r!r}; server alive afterwards: {alive}"); ok &= alive
srv.kill(); sys.exit(0 if ok else 1)
