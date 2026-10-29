-- Pandoc filter: fenced blocks of class `tezaur` are replaced by a cited answer from your TEZAUR server.
--   ```{.tezaur mode=ask}            (mode: ask | compare | gaps | search)
--   What is the burden of proof?
--   ```
-- Run: pandoc in.md --lua-filter tezaur.lua -o out.docx      Env: TEZAUR_SERVER, TEZAUR_TOKEN
-- Answers come from a local model: the filter makes document builds slow and non-reproducible; use it to draft, then freeze the text.
local server = os.getenv("TEZAUR_SERVER") or "http://127.0.0.1:8787"
local token = os.getenv("TEZAUR_TOKEN")

local function post(path, body)
  local args = { "-s", "--max-time", "900", "-H", "Content-Type: application/json", "-d", "@-" }
  if token then table.insert(args, "-H"); table.insert(args, "Authorization: Bearer " .. token) end
  table.insert(args, server:gsub("/$", "") .. path)
  local ok, out = pcall(pandoc.pipe, "curl", args, body)
  if not ok then return nil, "curl failed: " .. tostring(out) end
  local ok2, d = pcall(pandoc.json and pandoc.json.decode or function(s) error("no json") end, out)
  if not ok2 or type(d) ~= "table" then return nil, "unexpected answer from " .. server end
  return d
end

local function paras(text)
  local blocks = {}
  for p in (text .. "\n\n"):gmatch("(.-)\n\n") do
    if p:match("%S") then blocks[#blocks + 1] = pandoc.Para(pandoc.Inlines(pandoc.utils.stringify(p):gsub("\n", " "))) end
  end
  return blocks
end

function CodeBlock(cb)
  if not cb.classes:includes("tezaur") then return nil end
  local mode = cb.attributes.mode or "ask"
  local q = cb.text:gsub("^%s+", ""):gsub("%s+$", "")
  -- build the JSON by hand so we do not depend on a particular pandoc version's encoder
  local function esc(s) return (s:gsub('[%c"\\]', function(c) return string.format("\\u%04x", c:byte()) end)) end
  local d, err = post(mode == "search" and "/api/search" or "/api/ask", string.format('{"q":"%s","mode":"%s","k":%d}', esc(q), esc(mode), tonumber(cb.attributes.k) or 6))
  if not d then io.stderr:write("tezaur filter: " .. err .. "\n"); return pandoc.Para({ pandoc.Str("[TEZAUR unavailable: " .. err .. "]") }) end
  if d.error then return pandoc.Para({ pandoc.Str("[TEZAUR: " .. d.error .. "]") }) end
  local out = {}
  if d.answer then
    for _, b in ipairs(paras(d.answer)) do out[#out + 1] = b end
    if d.references and d.references ~= "" then out[#out + 1] = pandoc.CodeBlock(d.references) end
    for _, w in ipairs(d.warnings or {}) do io.stderr:write("tezaur filter warning: " .. w .. "\n") end
  else
    for _, h in ipairs(d.hits or {}) do
      out[#out + 1] = pandoc.BlockQuote({ pandoc.Para({ pandoc.Str(h.text:sub(1, 400)) }), pandoc.Para({ pandoc.Emph({ pandoc.Str(string.format("[%d] %s", h.n, h.label)) }) }) })
    end
  end
  return out
end
