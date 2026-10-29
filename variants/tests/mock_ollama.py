"""A fake Ollama (NDJSON /api/chat) for tests: always answers with a cited text that also contains one invented citation [9]."""
import json, http.server, sys
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers['Content-Length']); self.rfile.read(n)
        self.send_response(200); self.send_header('Content-Type', 'application/x-ndjson'); self.end_headers()
        if self.path == '/api/chat':
            for t in ["The burden of proof lies with the assessing body ", "[1]. Differences must be substantial ", "[2, 4]. Fabricated claim [9]."]:
                self.wfile.write((json.dumps({"message": {"content": t}, "done": False}) + "\n").encode())
            self.wfile.write((json.dumps({"message": {"content": ""}, "done": True}) + "\n").encode())
    def log_message(self, *a): pass
http.server.HTTPServer(('127.0.0.1', int(sys.argv[1]) if len(sys.argv) > 1 else 11999), H).serve_forever()
