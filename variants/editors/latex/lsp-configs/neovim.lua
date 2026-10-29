-- Neovim 0.11+
vim.lsp.config('tezaur', { cmd = { 'python3', 'TEZAUR_TOOLS/tezaur_lsp.py' }, filetypes = { 'markdown', 'tex', 'text' }, root_markers = { '.git' } })
vim.lsp.enable('tezaur')
