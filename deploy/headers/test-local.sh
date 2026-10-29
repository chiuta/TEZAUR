#!/bin/bash
# Local test of the three header configs against a stand-in upstream (no network needed). Needs caddy, nginx, openssl, curl; node+npm for the Cloudflare part.
# usage: test-local.sh SITE_DIR      (a directory with index.html)
set -u; exec 2>&1; D=$(cd "$(dirname "$0")" && pwd); SITE=${1:?site dir}; T=$(mktemp -d); fail=0
chk(){ if curl -sI --noproxy '*' "${@:2}" | grep -qiE "$1"; then echo "ok   $1"; else echo "FAIL $1"; echo "::error::FAIL $1"; fail=1; fi; }
(cd "$SITE" && python3 -m http.server 9001 >/dev/null 2>&1 &) ; sleep 1
sed -e 's#^tezaur.chiuta.ro, www.tezaur.chiuta.ro {#http://:9080 {#' -e '/reverse_proxy/,$d' "$D/Caddyfile" > $T/Caddyfile; printf '\treverse_proxy 127.0.0.1:9001\n}\n' >> $T/Caddyfile
caddy start --config $T/Caddyfile --adapter caddyfile >/dev/null 2>&1; sleep 2
echo "== caddy"; for h in x-frame-options:\ DENY x-content-type-options:\ nosniff strict-transport-security permissions-policy referrer-policy content-security-policy:.*frame-ancestors; do chk "^$h" -H 'Host: tezaur.chiuta.ro' localhost:9080/; done
chk "^location: https://tezaur.chiuta.ro/x" -H 'Host: www.tezaur.chiuta.ro' localhost:9080/x
caddy stop >/dev/null 2>&1
openssl req -x509 -newkey rsa:2048 -nodes -keyout $T/k.pem -out $T/c.pem -subj /CN=t -days 1 2>/dev/null
sed -e "s#listen 443 ssl http2;#listen 9443 ssl; ssl_certificate $T/c.pem; ssl_certificate_key $T/k.pem;#" -e 's#proxy_pass https://chiuta.github.io;#proxy_pass http://127.0.0.1:9001;#' -e '/proxy_ssl_server_name/d' "$D/nginx.conf" | { echo "events{}"; echo "http{"; cat; echo "}"; } > $T/nginx.conf
nginx -t -c $T/nginx.conf 2>&1 | grep -q successful || { echo "::error::FAIL nginx -t"; fail=1; }
nginx -c $T/nginx.conf -g "pid $T/n.pid; error_log $T/n.err;" || echo "::error::nginx start: $(cat $T/n.err 2>/dev/null | tail -2)"; sleep 1
echo "== nginx"; for h in x-frame-options:\ DENY x-content-type-options:\ nosniff strict-transport-security permissions-policy referrer-policy content-security-policy:.*frame-ancestors; do chk "^$h" -k https://localhost:9443/; done
chk "^location: https://tezaur.chiuta.ro/x" -k --resolve www.tezaur.chiuta.ro:9443:127.0.0.1 https://www.tezaur.chiuta.ro:9443/x
kill "$(cat $T/n.pid)" 2>/dev/null
if command -v npx >/dev/null; then echo "== cloudflare _headers (wrangler emulator)"; mkdir -p $T/cf/site/app && cp "$SITE/index.html" "$D/_headers" $T/cf/site/ && echo x > $T/cf/site/app/index.html
  (cd $T/cf && timeout 120 npx --yes wrangler@3 pages dev site --port 8788 --ip 127.0.0.1 >$T/wr.log 2>&1 &); for i in $(seq 60); do curl -s --noproxy '*' -o /dev/null http://127.0.0.1:8788/ && break; sleep 2; done
  chk "^x-frame-options: DENY" http://127.0.0.1:8788/; chk "^content-security-policy:.*frame-ancestors" http://127.0.0.1:8788/
  curl -sI --noproxy '*' http://127.0.0.1:8788/app/ | grep -qi "^content-security-policy" && { echo "FAIL /app/ must not get CSP"; fail=1; } || echo "ok   /app/ has no CSP"
fi
for p in 9001 8788; do pid=$(ss -ltnp 2>/dev/null | grep ":$p " | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2); [ -n "$pid" ] && kill $pid; done
exit $fail
