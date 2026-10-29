"""A fake local model for the semantic audit: answers SUPPORTS / CONTRADICTS / UNRELATED from simple rules
(negation mismatch, shared content words). Not intelligent — it only exercises the plumbing deterministically."""
import json, http.server, re, sys

def words(s):
    return {w[:5] for w in re.findall(r"[a-zăâîșț]{5,}", s.lower())}

class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers['Content-Length']); body = json.loads(self.rfile.read(n))
        user = body['messages'][-1]['content']
        m = re.search(r"PASSAGE:\n(.*)\n\nSENTENCE: (.*)", user, re.S)
        ans = "UNCLEAR"
        if m:
            passage, sent = m.group(1), m.group(2)
            neg = lambda t: bool(re.search(r"\b(not|never|no longer|cannot|nu)\b", t.lower()))
            shared = len(words(passage) & words(sent))
            if shared >= 2 and neg(passage) != neg(sent): ans = "CONTRADICTS"
            elif shared >= 2: ans = "SUPPORTS"
            else: ans = "UNRELATED"
        self.send_response(200); self.send_header('Content-Type', 'application/x-ndjson'); self.end_headers()
        self.wfile.write((json.dumps({"message": {"content": ans}, "done": False}) + "\n").encode())
        self.wfile.write((json.dumps({"message": {"content": ""}, "done": True}) + "\n").encode())
    def log_message(self, *a): pass

http.server.HTTPServer(('127.0.0.1', int(sys.argv[1]) if len(sys.argv) > 1 else 11998), H).serve_forever()
