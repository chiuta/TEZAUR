# TEZAUR over Tor / Tailscale / Yggdrasil (online peers without opening ports)

**Tor onion service** (works behind any NAT, hides the address): in `/etc/tor/torrc` on the node that SHARES:
```
HiddenServiceDir /var/lib/tor/tezaur/
HiddenServicePort 80 127.0.0.1:8787
```
`cat /var/lib/tor/tezaur/hostname` gives `xxxx.onion`. On the node that READS:
```
tezaur mesh add ana http://xxxx.onion --token SECRET --proxy socks5h://127.0.0.1:9050
tezaur mesh search "burden of proof"
```
**Tailscale / WireGuard / Yggdrasil**: just use the private address as the peer URL; no proxy needed.
Always set `--token` on a server reachable by others. Peers see only what you `mesh share`.
