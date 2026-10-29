(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs '((markdown-mode latex-mode text-mode) . ("python3" "TEZAUR_TOOLS/tezaur_lsp.py"))))
