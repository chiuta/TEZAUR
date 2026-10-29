-- TEZAUR for Neovim 0.10+: cited answers from your own library. Needs curl and `tezaur serve`.
local M = { cfg = { server = "http://127.0.0.1:8787", token = nil, k = 6 }, last = {} }

function M.setup(o) M.cfg = vim.tbl_extend("force", M.cfg, o or {}) end

--- Pure: server JSON -> display lines.
function M.format(mode, d)
  if d.error then return { "Error: " .. d.error } end
  local L = {}
  if mode == "search" then
    for _, h in ipairs(d.hits or {}) do
      L[#L + 1] = string.format("[%d] %s", h.n, h.label)
      L[#L + 1] = "    " .. (h.text:sub(1, 300):gsub("%s+", " "))
    end
    if #L == 0 then L[1] = "No matching passages." end
    return L
  end
  for line in (d.answer .. "\n"):gmatch("(.-)\n") do L[#L + 1] = line end
  if d.references and d.references ~= "" then
    L[#L + 1] = ""; L[#L + 1] = "## References"
    for line in d.references:gmatch("[^\n]+") do L[#L + 1] = line end
  end
  for _, w in ipairs(d.warnings or {}) do L[#L + 1] = "⚠ " .. w end
  return L
end

local function show(lines)
  vim.cmd("botright 14new")
  local buf = vim.api.nvim_get_current_buf()
  vim.bo[buf].buftype, vim.bo[buf].bufhidden, vim.bo[buf].swapfile, vim.bo[buf].filetype = "nofile", "wipe", false, "markdown"
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].modifiable = false
end

function M.run(mode, q, on_done)
  q = vim.trim(q or "")
  if q == "" then return vim.notify("TEZAUR: empty question", vim.log.levels.WARN) end
  vim.notify("TEZAUR: thinking… (a local model can take a minute)")
  local cmd = { "curl", "-s", "--max-time", "600", "-H", "Content-Type: application/json", "-d", "@-" }
  if M.cfg.token then vim.list_extend(cmd, { "-H", "Authorization: Bearer " .. M.cfg.token }) end
  table.insert(cmd, M.cfg.server:gsub("/$", "") .. (mode == "search" and "/api/search" or "/api/ask"))
  vim.system(cmd, { text = true, stdin = vim.json.encode({ q = q, mode = mode, k = M.cfg.k }) }, vim.schedule_wrap(function(r)
    local ok, d = pcall(vim.json.decode, r.stdout or "")
    if r.code ~= 0 or not ok then d = { error = "cannot reach " .. M.cfg.server } end
    M.last = M.format(mode, d)
    if on_done then on_done(M.last, d) else show(M.last) end
  end))
end

function M.selection()
  local a, b = vim.fn.getpos("'<"), vim.fn.getpos("'>")
  return table.concat(vim.api.nvim_buf_get_text(0, a[2] - 1, a[3] - 1, b[2] - 1, b[3], {}), " ")
end

function M.insert()
  if #M.last == 0 then return vim.notify("TEZAUR: nothing to insert yet") end
  local row = vim.api.nvim_win_get_cursor(0)[1]
  vim.api.nvim_buf_set_lines(0, row, row, false, M.last)
end

return M
