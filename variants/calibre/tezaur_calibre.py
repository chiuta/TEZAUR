#!/usr/bin/env python3
"""Sends your Calibre library to TEZAUR: uploads each book's best text format (EPUB > PDF > DOCX > TXT) to `tezaur serve`
and writes a BibTeX file whose keys equal the uploaded file names, so citations read "Author (Year)" instead of a file name.
Needs Calibre's `calibredb` on PATH (or --json FILE with the output of `calibredb list --for-machine --fields ...`).

    python3 tezaur_calibre.py --library ~/Calibre --server http://127.0.0.1:8787 --bib-out ~/thesis/calibre.bib [--tag thesis]
(Copy calibre.bib next to the TEZAUR index, or into Data/ for the portable build.)  Re-running skips books already uploaded."""
import argparse, json, os, re, subprocess, sys, unicodedata, urllib.parse, urllib.request

PREFERENCE = ["epub", "pdf", "docx", "txt", "md", "html"]
FIELDS = "id,title,authors,pubdate,publisher,formats,identifiers,tags"


def slug(s):
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode()
    return re.sub(r"[^A-Za-z0-9]+", "_", s).strip("_").lower()[:60] or "book"


def key_for(book):
    first = (book.get("authors") or "anon").split(" & ")[0].split()[-1]
    year = (book.get("pubdate") or "")[:4]
    return slug(f"{first}{year}_{book.get('title', '')[:30]}")


def pick_format(book):
    by_ext = {os.path.splitext(f)[1].lstrip(".").lower(): f for f in book.get("formats", [])}
    for ext in PREFERENCE:
        if ext in by_ext:
            return by_ext[ext]
    return None


def bibtex(book, key):
    esc = lambda v: v.replace("{", "(").replace("}", ")")
    authors = " and ".join(a.strip() for a in (book.get("authors") or "").split(" & ") if a.strip())
    doi = (book.get("identifiers") or {}).get("doi", "")
    lines = [f"@book{{{key},", f"  title = {{{esc(book.get('title', ''))}}},"]
    if authors: lines.append(f"  author = {{{esc(authors)}}},")
    if (book.get("pubdate") or "")[:4].isdigit(): lines.append(f"  year = {{{book['pubdate'][:4]}}},")
    if book.get("publisher"): lines.append(f"  publisher = {{{esc(book['publisher'])}}},")
    if doi: lines.append(f"  doi = {{{doi}}},")
    return "\n".join(lines) + "\n}\n"


def upload(server, token, path, name, post=urllib.request.urlopen):
    with open(path, "rb") as f:
        req = urllib.request.Request(f"{server}/api/ingest?name={urllib.parse.quote(name)}", f.read(), method="POST")
    if token: req.add_header("Authorization", "Bearer " + token)
    return json.load(post(req, timeout=900))


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--library"); ap.add_argument("--json"); ap.add_argument("--tag")
    ap.add_argument("--server", default=os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787")); ap.add_argument("--token", default=os.environ.get("TEZAUR_TOKEN"))
    ap.add_argument("--bib-out", default="calibre.bib"); ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args(argv)
    if a.json:
        books = json.load(open(a.json, encoding="utf-8"))
    else:
        cmd = ["calibredb", "list", "--for-machine", "--fields", FIELDS, "--limit", "100000"] + (["--with-library", a.library] if a.library else []) + (["--search", f"tags:\"={a.tag}\""] if a.tag else [])
        try:
            books = json.loads(subprocess.run(cmd, capture_output=True, text=True, check=True).stdout)
        except FileNotFoundError:
            sys.exit("calibredb not found (install Calibre, or pass --json)")
    entries, sent, skipped = [], 0, 0
    for b in books:
        f = pick_format(b)
        if not f or not os.path.exists(f):
            skipped += 1; continue
        key = key_for(b)
        entries.append(bibtex(b, key))
        if a.dry_run: continue
        try:
            r = upload(a.server.rstrip("/"), a.token, f, key + os.path.splitext(f)[1].lower())
            sent += r.get("status") == "added"
            print(f"{key}: {r.get('status')}")
        except Exception as e:
            print(f"{key}: failed ({e})", file=sys.stderr)
    open(a.bib_out, "w", encoding="utf-8").write("\n".join(entries))
    print(f"{len(entries)} books, {sent} newly uploaded, {skipped} without a usable file; BibTeX in {a.bib_out}")


if __name__ == "__main__":
    main()
