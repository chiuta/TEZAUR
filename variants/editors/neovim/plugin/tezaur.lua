local t = require("tezaur")
for name, mode in pairs({ TezaurAsk = "ask", TezaurCompare = "compare", TezaurGaps = "gaps", TezaurSearch = "search" }) do
  vim.api.nvim_create_user_command(name, function(o) t.run(mode, o.args) end, { nargs = "+" })
end
vim.api.nvim_create_user_command("TezaurAskSelection", function() t.run("ask", t.selection()) end, { range = true })
vim.api.nvim_create_user_command("TezaurInsert", function() t.insert() end, {})
