"""Turn the TSV written by `tezaur flashcards -o cards.tsv` into an Anki package (.apkg)."""
import argparse
import hashlib
import sys


def read_cards(path):
    cards = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            front, _, back = line.partition("\t")
            if front and back:
                cards.append((front, back))
    return cards


def stable_id(name, salt):
    return int(hashlib.sha1((salt + name).encode()).hexdigest()[:8], 16) + 1_000_000_000


def build(cards, out, deck="TEZAUR"):
    try:
        import genanki
    except ImportError:
        sys.exit("pip install genanki   (or: pip install 'tezaur-client[anki]')")
    model = genanki.Model(stable_id("model", "m"), "TEZAUR card",
                          fields=[{"name": "Question"}, {"name": "Answer"}],
                          templates=[{"name": "Card 1", "qfmt": "{{Question}}", "afmt": "{{FrontSide}}<hr id=answer>{{Answer}}"}])
    d = genanki.Deck(stable_id(deck, "d"), deck)
    esc = lambda s: s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    for front, back in cards:
        # guid from the question: re-importing an updated file updates cards instead of duplicating them
        d.add_note(genanki.Note(model=model, fields=[esc(front), esc(back)], guid=genanki.guid_for(deck, front), tags=["tezaur"]))
    genanki.Package(d).write_to_file(out)
    return len(cards)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="tezaur-apkg")
    ap.add_argument("tsv")
    ap.add_argument("-o", "--output", default="cards.apkg")
    ap.add_argument("--deck", default="TEZAUR")
    a = ap.parse_args(argv)
    n = build(read_cards(a.tsv), a.output, a.deck)
    print("wrote %s (%d cards)" % (a.output, n))


if __name__ == "__main__":
    main()
