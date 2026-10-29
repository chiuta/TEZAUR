Windows only: copy `dist/TezaurPortable/*` next to this manifest plus `Assets/` (logo.png 50x50, logo44.png, logo150.png), then
`makeappx pack /d . /p Tezaur.msix` and sign with `signtool` (an MSIX must be signed; a Store submission needs a Partner Center account).
Not built or validated here (no Windows tooling in the sandbox); the XML is only checked for well-formedness.
