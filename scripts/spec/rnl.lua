--[[
Pandoc filter for the RNL standard, the same for the web page (html5) and the PDF (typst):

- headings get their section number as identifier: "9.4", "A", "A.1" (not those of the principles, which the
  web page shows after the annexes, in a div of the class principles: their ids start with principles, and in
  it the numbers 1 to 4 are not linked to the standard's sections);
- section references in the text, such as "(9.4)", "chapter 7", "Annex A", "第 7 章", "附录 A",
  become links; "Principles 2.4" and "原则 2.4" refer to the principles and stay text;
- code blocks marked rnl, spice, spectre or rnl-file are coloured by one tokenizer for both outputs;
  blocks marked error get the label of an error example;
- on a page that compares two versions (specdiff.py), a code block with data-diff holds the lines of
  both, each coloured as it reads in its own version and marked as kept, removed or added;
- for Typst, the table of contents is put before chapter 1, after the front matter.

Metadata read: rnl-lang ("en" or "zh"). Linked references are listed on stderr with --verbose-links.
]]

local is_typst = FORMAT:match("typst") ~= nil
local lang = "en"
local ids = {}
local in_principles = false  -- inside the principles, whose own numbers, 1 to 4, are not those of the standard's sections
local verbose = os.getenv("RNL_LINKS_LOG")
local log = verbose and io.open(verbose, "a") or nil

local TEXT = {
  en = { error = "Error example", contents = "Contents" },
  zh = { error = "错例", contents = "目录" },
}

-- Typst labels are written <…>; keep them plain and unambiguous.
local function anchor(id)
  return is_typst and ("sec-" .. id) or id
end

local function section_id(text)
  local n = text:match("^(%d+[%.%d]*)%s") or text:match("^(%d+)$")
  if n then return (n:gsub("%.$", "")) end
  local a = text:match("^Annex (%u)%s") or text:match("^附录 (%u)%s")
  if a then return a end
  return text:match("^(%u%.%d+)%s")
end

------------------------------------------------------------------------------------------------
-- Pass 1: metadata and section identifiers

local function read_meta(meta)
  if meta["rnl-lang"] then lang = pandoc.utils.stringify(meta["rnl-lang"]) end
end

local function set_header_id(h)
  if h.identifier:match("^principles") then return h end
  local id = section_id(pandoc.utils.stringify(h.content))
  if id then
    h.identifier = anchor(id)
    ids[id] = true
  end
  return h
end

------------------------------------------------------------------------------------------------
-- Pass 2: links for section references

local NO_LINK_AFTER = { "Principles$", "原则$", "[Vv]ersion$", "[Vv]ersions$", "版本$", "RNL$", "before$" }

local function blocked(prev)
  if not prev then return false end
  for _, p in ipairs(NO_LINK_AFTER) do
    if prev:match(p) then return true end
  end
  return false
end

local function neighbour(inlines, i, step)
  local j = i + step
  if inlines[j] and inlines[j].t == "Space" then j = j + step end
  local el = inlines[j]
  if el and el.t == "Str" then return el.text end
  return nil
end

local function link(id, text)
  if log then log:write(lang, "\t", id, "\t", text, "\n") end
  return pandoc.Link({ pandoc.Str(text) }, "#" .. anchor(id))
end

-- Split one Str into text and links. prev and next are the neighbouring words; chain is true when the
-- word comes in a list after "Principles" ("Principles 2.4, 2.9"), whose numbers all name principles.
-- Returns the new inlines (or nil when nothing is linked) and whether the list goes on.
local function link_str(s, prev, nxt, chain)
  local out = {}
  local pos = 1
  local function emit(upto)
    if upto >= pos then table.insert(out, pandoc.Str(s:sub(pos, upto))) end
  end
  -- chapter 7 / chapters 9–17 / 第 7 章
  local ch = s:match("^(%d+)")
  if ch and ids[ch] and prev and (prev:match("[Cc]hapters?$") or (prev:match("第$") and nxt and nxt:match("^章"))) then
    table.insert(out, link(ch, ch))
    pos = #ch + 1
  end
  -- Annex A / 附录 A
  local an = s:match("^(%u)")
  if pos == 1 and an and ids[an] and prev and (prev:match("Annex$") or prev:match("附录$"))
     and not s:sub(2, 2):match("[%w]") then
    table.insert(out, link(an, an))
    pos = 2
  end
  -- dotted section numbers, anywhere in the word: (9.4), 9.4, （5.3、附录
  local init = pos
  local last = 0  -- where the last number ended
  while true do
    local a, b = s:find("%d+%.%d+[%.%d]*", init)
    if not a then break end
    local num = s:sub(a, b)
    local trimmed = num:gsub("%.+$", "")
    local gap = s:sub(last + 1, a - 1)
    if chain and not (gap:match("^[%s,、]*$") or (last == 0 and gap:match("^[%(（]*$"))) then chain = false end
    if (a == 1 and blocked(prev)) or (a > 1 and blocked(s:sub(1, a - 1))) then chain = true end
    local before = a > 1 and s:sub(a - 1, a - 1) or ""
    local after = s:sub(a + #trimmed, a + #trimmed)
    local ok = ids[trimmed] and not chain and not before:match("[%w%.]") and not after:match("[%w]")
    if ok and in_principles and tonumber(trimmed:match("^%d+")) <= 4 then ok = false end
    if ok then
      emit(a - 1)
      table.insert(out, link(trimmed, trimmed))
      pos = a + #trimmed
    end
    last = a + #trimmed - 1
    init = a + #trimmed
  end
  local goes_on = (chain and last > 0 and s:sub(last + 1):match("^[,、]$") ~= nil) or blocked(s)
  if pos == 1 then return nil, goes_on end
  emit(#s)
  return out, goes_on
end

local LIST_WORDS = { ["and"] = true, ["or"] = true, ["和"] = true, ["或"] = true }

local function link_inlines(inlines)
  local result = pandoc.Inlines({})
  local chain = false
  for i, el in ipairs(inlines) do
    if el.t == "Str" then
      if chain and LIST_WORDS[el.text] then
        result:insert(el)
      else
        local parts
        parts, chain = link_str(el.text, neighbour(inlines, i, -1), neighbour(inlines, i, 1), chain)
        if parts then
          for _, p in ipairs(parts) do result:insert(p) end
        else
          result:insert(el)
        end
      end
    else
      if el.t ~= "Space" and el.t ~= "SoftBreak" then chain = false end
      result:insert(el)
    end
  end
  return result
end

------------------------------------------------------------------------------------------------
-- Code: one tokenizer, two outputs

local NAME = "[%w_\128-\255]"
local REFCH = "[%w_<>%[%]:%*\128-\255]"
local SIGIL = { ["#"] = "i", ["&"] = "n", ["%"] = "v", ["$"] = "d", ["?"] = "r" }

-- Tokens of RNL text: a list of {class, text}. Classes: k keyword, i n v d r references
-- (# & % $ ?), s string, c comment, o operator, t free text, p parameters, pt a point,
-- r also a role in brackets, "" white space.
local function tokenize_rnl(s, out)
  local i, n = 1, #s
  local paren = 0
  local role = false
  local pkind = "p"  -- what the brackets hold: parameters of a keyword, a role, or a point
  local function add(cls, text) table.insert(out, { cls, text }) end
  while i <= n do
    local c = s:sub(i, i)
    local j
    if s:sub(i, i + 1) == "/*" then
      local e = s:find("*/", i + 2, true)
      j = e and e + 1 or n
      add("c", s:sub(i, j))
    elseif c == '"' then
      j = i + 1
      while j <= n do
        if s:sub(j, j) == '"' then
          if s:sub(j + 1, j + 1) == '"' then j = j + 2 else break end
        else
          j = j + 1
        end
      end
      add("s", s:sub(i, j))
    elseif c == "@" and s:sub(i + 1, i + 1):match(NAME) then
      j = i + 1
      while j + 1 <= n and s:sub(j + 1, j + 1):match("[%w_%-\128-\255]") do j = j + 1 end
      add("k", s:sub(i, j))
    elseif SIGIL[c] then
      j = i
      if c == "%" and s:sub(j + 1, j + 1) == "#" then j = j + 1 end  -- a number, such as %#3 (5.9 of 0.3)
      if s:sub(j + 1, j + 1) == '"' then
        local e = s:find('"', j + 2, true)
        j = e or n
      end
      while j + 1 <= n do
        local ch = s:sub(j + 1, j + 1)
        if ch:match(REFCH) then
          j = j + 1
        elseif ch == '"' and s:sub(j - 1, j) == "::" then
          local e = s:find('"', j + 2, true)
          j = e or n
        else
          break
        end
      end
      add(SIGIL[c], s:sub(i, j))
      if s:sub(j + 1, j + 1) == "(" and paren == 0 then role = true end
    elseif s:sub(i, i + 1) == "=>" then
      j = i + 1
      add("o", "=>")
    elseif c:match("[;{}=]") and paren == 0 then
      j = i
      add("o", c)
    elseif c == "(" then
      j = i
      if paren == 0 then
        local last = out[#out]
        if role then pkind = "r" elseif last and last[1] == "k" then pkind = "p" else pkind = "pt" end
      end
      paren = paren + 1
      add(pkind, c)
    elseif c == ")" then
      j = i
      paren = math.max(paren - 1, 0)
      add(pkind, c)
      if paren == 0 then role = false; pkind = "p" end
    elseif c:match("%s") then
      j = i
      while j + 1 <= n and s:sub(j + 1, j + 1):match("%s") do j = j + 1 end
      add("", s:sub(i, j))
    else
      j = i
      while j + 1 <= n do
        local ch = s:sub(j + 1, j + 1)
        if ch:match("[%s\"@#&%%%$%?;{}=()]") or s:sub(j + 1, j + 2) == "/*" then break end
        j = j + 1
      end
      add(paren > 0 and pkind or "t", s:sub(i, j))
    end
    i = j + 1
  end
end

local function brace_depth(tokens, depth)
  for _, t in ipairs(tokens) do
    if t[1] == "o" then
      if t[2] == "{" then depth = depth + 1 elseif t[2] == "}" then depth = depth - 1 end
    end
  end
  return depth
end

-- A host netlist with RNL in its comments (spice, rnl-file), or RNL with carrier lines.
local function tokenize_host(text, all_rnl)
  local out = {}
  local depth = 0
  local first = true
  for line in (text .. "\n"):gmatch("(.-)\n") do
    if not first then table.insert(out, { "", "\n" }) end
    first = false
    local prefix, rest = line:match("^(%*%s?)(.*)$")
    if not prefix then prefix, rest = line:match("^(//%s?)(.*)$") end
    if prefix then
      local at = rest:find("@RNL", 1, true)
      if depth > 0 or at or all_rnl then
        table.insert(out, { "c", prefix })
        local toks = {}
        if depth == 0 and at and at > 1 then
          table.insert(out, { "c", rest:sub(1, at - 1) })
          rest = rest:sub(at)
        end
        tokenize_rnl(rest, toks)
        depth = math.max(brace_depth(toks, depth), 0)
        for _, t in ipairs(toks) do table.insert(out, t) end
      else
        table.insert(out, { "c", line })
      end
    elseif all_rnl then
      local toks = {}
      tokenize_rnl(line, toks)
      for _, t in ipairs(toks) do table.insert(out, t) end
    elseif line:match("^%s*%.") then
      table.insert(out, { "h", line })
    else
      local name, tail = line:match("^(%S+)(.*)$")
      if name then
        table.insert(out, { "hi", name })
        table.insert(out, { "h", tail })
      else
        table.insert(out, { "", line })
      end
    end
  end
  return out
end

local function lines_of(tokens)
  local lines = { {} }
  for _, t in ipairs(tokens) do
    local cls, text = t[1], t[2]
    local start = 1
    while true do
      local nl = text:find("\n", start, true)
      local piece = nl and text:sub(start, nl - 1) or text:sub(start)
      if #piece > 0 then table.insert(lines[#lines], { cls, piece }) end
      if not nl then break end
      table.insert(lines, {})
      start = nl + 1
    end
  end
  return lines
end

local function html_escape(s)
  return (s:gsub("&", "&amp;"):gsub("<", "&lt;"):gsub(">", "&gt;"):gsub('"', "&quot;"))
end

local function typst_string(s)
  return '"' .. s:gsub("\\", "\\\\"):gsub('"', '\\"') .. '"'
end

-- marks: for a block compared with an earlier version, one character per line: "=" kept, "-" removed, "+" added
local function render(lines, kind, is_error, marks)
  if is_typst then
    local L = {}
    for _, line in ipairs(lines) do
      local segs = {}
      for _, t in ipairs(line) do
        table.insert(segs, "(" .. typst_string(t[1]) .. ", " .. typst_string(t[2]) .. "),")
      end
      table.insert(L, "(" .. table.concat(segs, " ") .. "),")
    end
    local label = is_error and typst_string(TEXT[lang].error) or "none"
    return pandoc.RawBlock("typst", "#rnlcode(label: " .. label .. ", (\n" .. table.concat(L, "\n") .. "\n))")
  end
  local H = {}
  local MARK = { ["-"] = "d-del", ["+"] = "d-ins" }
  for n, line in ipairs(lines) do
    if n > 1 then table.insert(H, "\n") end
    local mark = marks and MARK[marks:sub(n, n)]
    if mark then table.insert(H, '<span class="' .. mark .. '">') end
    for _, t in ipairs(line) do
      if t[1] == "" then
        table.insert(H, html_escape(t[2]))
      else
        table.insert(H, '<span class="t-' .. t[1] .. '">' .. html_escape(t[2]) .. "</span>")
      end
    end
    if mark then table.insert(H, "</span>") end
  end
  local pre = '<pre class="code ' .. kind .. '"><code>' .. table.concat(H) .. "</code></pre>"
  if is_error then
    pre = '<figure class="code-error"><figcaption>' .. TEXT[lang].error .. "</figcaption>" .. pre .. "</figure>"
  end
  return pandoc.RawBlock("html", pre)
end

-- The lines of tokens of a code block's text, coloured when it is marked rnl, spice or rnl-file.
local function code_lines(kind, text)
  if kind == "rnl" then
    if text:match("^%s*%*") or text:match("\n%s*%*") then return lines_of(tokenize_host(text, true)) end
    local toks = {}
    tokenize_rnl(text, toks)
    return lines_of(toks)
  elseif kind == "spice" or kind == "spectre" or kind == "rnl-file" then
    return lines_of(tokenize_host(text, false))
  end
  return lines_of({ { "", text } })
end

-- A block compared with an earlier version: the old lines and the new lines are each coloured as one
-- text, so that a record over several lines reads as in its own version, and then interleaved again.
local function diff_block(cb, marks)
  local old, new = {}, {}
  local n = 0
  for line in (cb.text .. "\n"):gmatch("(.-)\n") do
    n = n + 1
    local m = marks:sub(n, n)
    if m ~= "+" then table.insert(old, line) end
    if m ~= "-" then table.insert(new, line) end
  end
  local kind = cb.classes[1]
  local lo, ln = code_lines(kind, table.concat(old, "\n")), code_lines(kind, table.concat(new, "\n"))
  local lines, i, j = {}, 1, 1
  for k = 1, #marks do
    local m = marks:sub(k, k)
    if m == "-" then
      table.insert(lines, lo[i]); i = i + 1
    else
      table.insert(lines, ln[j]); j = j + 1
      if m == "=" then i = i + 1 end
    end
  end
  return render(lines, kind or "text", cb.classes:includes("error"), marks)
end

local function code_block(cb)
  if cb.attributes["data-diff"] then return diff_block(cb, cb.attributes["data-diff"]) end
  local kind = cb.classes[1]
  local is_error = cb.classes:includes("error")
  if kind == "rnl" then
    local carrier = cb.text:match("^%s*%*") or cb.text:match("\n%s*%*")
    local toks
    if carrier then
      toks = tokenize_host(cb.text, true)
    else
      toks = {}
      tokenize_rnl(cb.text, toks)
    end
    return render(lines_of(toks), "rnl", is_error)
  elseif kind == "spice" or kind == "spectre" or kind == "rnl-file" then
    return render(lines_of(tokenize_host(cb.text, false)), kind == "spectre" and "spice" or kind, is_error)
  end
  if is_error then
    return render(lines_of({ { "", cb.text } }), kind or "text", true)
  end
  return nil
end

------------------------------------------------------------------------------------------------
-- Typst: the table of contents after the front matter

local function place_outline(doc)
  if not is_typst then return nil end
  for i, b in ipairs(doc.blocks) do
    if b.t == "Header" and b.identifier == anchor("1") then
      doc.blocks:insert(i, pandoc.RawBlock("typst",
        "#outline(title: [" .. TEXT[lang].contents .. "], depth: 2, indent: auto)\n#pagebreak()"))
      return doc
    end
  end
  return nil
end

local links = {
  traverse = "topdown",
  Header = function(h) return h, false end,
  Link = function(l) return l, false end,
  Code = function(c) return c, false end,
  CodeBlock = code_block,
  Inlines = link_inlines,
}

-- The principles are walked with the same filter, apart from the rule about their own numbers.
local in_principles_div = {
  traverse = "topdown",
  Header = links.Header, Link = links.Link, Code = links.Code, CodeBlock = code_block, Inlines = link_inlines,
}

return {
  { Meta = read_meta },
  { Header = set_header_id },
  {
    traverse = "topdown",
    Div = function(d)
      if not d.classes:includes("principles") then return nil end
      in_principles = true
      local walked = pandoc.walk_block(d, in_principles_div)
      in_principles = false
      return walked, false
    end,
    Header = links.Header,
    Link = links.Link,
    Code = links.Code,
    CodeBlock = code_block,
    Inlines = link_inlines,
  },
  { Pandoc = place_outline },
}
