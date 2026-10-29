"""A fake OpenAlex + Crossref on one port (paths do not collide), for testing `tezaur lit` and `refs --online` offline."""
import json, http.server, sys, urllib.parse

OA = {
 "10.1000/real":  {"title": "Mutual recognition of professional qualifications", "publication_year": 2019, "doi": "https://doi.org/10.1000/real", "is_retracted": False, "cited_by_count": 42,
                   "authorships": [{"author": {"display_name": "Ana Popescu"}}, {"author": {"display_name": "Ion Ionescu"}}], "primary_location": {"source": {"display_name": "Journal of EU Law"}}},
 "10.1000/retr":  {"title": "A miracle cure for everything", "publication_year": 2018, "doi": "https://doi.org/10.1000/retr", "is_retracted": True, "cited_by_count": 900,
                   "authorships": [{"author": {"display_name": "Zed Quack"}}], "primary_location": {"source": {"display_name": "Annals of Doubt"}}},
 "10.1000/year":  {"title": "Free movement of workers in the single market", "publication_year": 2015, "doi": "https://doi.org/10.1000/year", "is_retracted": False, "cited_by_count": 3,
                   "authorships": [{"author": {"display_name": "Maria Rossi"}}], "primary_location": {"source": {"display_name": "EU Review"}}},
}
CR_ONLY = {"10.2000/cr": {"title": ["Only in Crossref"], "DOI": "10.2000/cr", "issued": {"date-parts": [[2020]]}, "author": [{"given": "C", "family": "Rossi"}], "container-title": ["CR Journal"]}}

class H(http.server.BaseHTTPRequestHandler):
    def send(self, code, obj):
        b = json.dumps(obj).encode(); self.send_response(code); self.send_header('Content-Type', 'application/json'); self.end_headers(); self.wfile.write(b)
    def do_GET(self):
        u = urllib.parse.urlparse(self.path); q = urllib.parse.parse_qs(u.query); p = urllib.parse.unquote(u.path)
        if p == "/works" and "search" in q:       # OpenAlex search
            t = q["search"][0].lower()
            return self.send(200, {"results": [w for w in OA.values() if any(x in w["title"].lower() for x in t.split())]})
        if p == "/works" and "query" in q:        # Crossref search
            return self.send(200, {"message": {"items": [v for v in CR_ONLY.values()]}})
        if p.startswith("/works/https://doi.org/"):   # OpenAlex by DOI
            w = OA.get(p[len("/works/https://doi.org/"):]); return self.send(200, w) if w else self.send(404, {})
        if p.startswith("/works/"):               # Crossref by DOI
            w = CR_ONLY.get(p[len("/works/"):]); return self.send(200, {"message": w}) if w else self.send(404, {})
        self.send(404, {})
    def log_message(self, *a): pass

http.server.HTTPServer(('127.0.0.1', int(sys.argv[1]) if len(sys.argv) > 1 else 11997), H).serve_forever()
