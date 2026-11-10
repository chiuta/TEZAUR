import http.server,socketserver,sys,time
MODE=sys.argv[1]; PORT=int(sys.argv[2])
HDR='<?xml version="1.0"?>'
def rec(i,extra=''):
    return f'<record><header><identifier>oai:x:{i}</identifier><datestamp>2020-01-01</datestamp></header><metadata><oai_dc:dc xmlns:oai_dc="http://www.openarchives.org/OAI/2.0/oai_dc/" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>T{i}{extra}</dc:title><dc:creator>A, B</dc:creator><dc:date>2020</dc:date><dc:identifier>10.1/x{i}</dc:identifier></oai_dc:dc></metadata></record>'
def wrap(body,tok=''):
    return f'{HDR}<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/"><ListRecords>{body}{tok}</ListRecords></OAI-PMH>'
class H(http.server.BaseHTTPRequestHandler):
    n=0
    def log_message(self,*a): pass
    def do_GET(self):
        H.n+=1; body=None; code=200
        if MODE=='bomb':
            ents=''.join(f'<!ENTITY e{i} "{"&e%d;"%(i-1)*10}">' for i in range(1,10)); 
            body=HDR+f'<!DOCTYPE x [<!ENTITY e0 "lol">{ents}]>'+wrap(rec(1,'&e9;'))
        elif MODE=='deep': body=wrap(rec(1,'<a>'*100000+'</a>'*100000))
        elif MODE=='loop': body=wrap(rec(H.n),'<resumptionToken>SAME</resumptionToken>')
        elif MODE=='endless': body=wrap(rec(H.n),f'<resumptionToken>t{H.n}</resumptionToken>')
        elif MODE=='emptypages': body=wrap('',f'<resumptionToken>t{H.n}</resumptionToken>')
        elif MODE=='big': body=wrap(rec(1,'A'*30_000_000))
        elif MODE=='tags': body=wrap(rec(1,'<b'+' x="1"'*200000+'/>'))
        elif MODE=='ctrl': body=wrap(rec(1,'\x00‮\x1b[31m<script>alert(1)</script>'))
        elif MODE=='slow':
            self.send_response(200); self.end_headers(); 
            for _ in range(600): self.wfile.write(b' '); self.wfile.flush(); time.sleep(1)
            return
        elif MODE=='redirect': self.send_response(302); self.send_header('Location','http://169.254.169.254/latest/meta-data/'); self.end_headers(); return
        elif MODE=='cdata': body=wrap(rec(1,'<![CDATA[]]]]><![CDATA[>'+']]>'*50000))
        b=body.encode('utf8','replace'); self.send_response(code); self.send_header('Content-Type','text/xml'); self.send_header('Content-Length',str(len(b))); self.end_headers(); self.wfile.write(b)
class S(socketserver.ThreadingMixIn,http.server.HTTPServer): daemon_threads=True
S(('127.0.0.1',PORT),H).serve_forever()
