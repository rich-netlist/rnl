/* RNL introduction page · code highlighting and code panels
 *
 * Display-only highlighting, not a parser: host netlist lines, ordinary comment lines and @RNL
 * records (which may span lines) are handled separately.
 * Inside a record, tokens are colored by the language draft's notation: keywords @PLACE (and their
 * aliases, such as the Chinese ones) with their parameters in tight parentheses, #instance &net %name $definition ?role,
 * a (role) tight after an object, points (x, y), sets {…}, the pipe =>; everything else is free text.
 */
(function (global) {
  'use strict';

  function esc(s) {
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function span(cls, s) { return '<span class="' + cls + '">' + esc(s) + '</span>'; }

  /* ---------- Host netlist (a structural subset of SPICE) ---------- */

  function hlSpiceRest(rest) {
    return rest.replace(/(\s+)|([A-Za-z_]\w*)=(\{[^}]*\}|\S+)|(\S+)/g, function (all, ws, pn, pv, tok) {
      if (ws) return ws;
      if (pn) return span('t-pn', pn) + '=' + span('t-hval', pv);
      if (/^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?[a-z]*$/i.test(tok)) return span('t-hval', tok);
      return span('t-hnet', tok);
    });
  }

  function hlSpice(text) {
    var m = /^(\s*)(\S+)(.*)$/.exec(text);
    if (!m) return esc(text);
    var head = m[2].charAt(0) === '.' ? span('t-dir', m[2]) : span('t-dev', m[2]);
    return esc(m[1]) + head + hlSpiceRest(m[3]);
  }

  /* ---------- RNL records ---------- */

  var Q = '"(?:[^"]|"")*"';
  var NAME = '(?:' + Q + '|[\\p{L}\\p{N}_]+)';
  var SUB = '(?:<[^<>\\s]*>)?';
  var KIDS = '(?:::' + NAME + ')*';
  var RE = {
    ws: /\s+/y,
    str: /"(?:[^"]|"")*"?/y,
    kw: /@[\p{L}\p{N}_]+(?:-[\p{L}\p{N}_]+)*/uy,
    inst: new RegExp('#' + NAME + '?' + SUB + '(?::' + NAME + SUB + ')?' + KIDS, 'uy'),
    net: new RegExp('&' + NAME + SUB + KIDS, 'uy'),
    obj: new RegExp('%' + NAME + '(?::' + NAME + ')?' + KIDS, 'uy'),
    def: new RegExp('\\$' + NAME, 'uy'),
    slot: new RegExp('\\?' + NAME + '?' + KIDS + '(?::' + NAME + ')?', 'uy'),
    pt: /\((?:[^()"\n]|"(?:[^"]|"")*")*\)/y,
    num: /[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/y,
    ident: /[\p{L}_][\p{L}\p{N}_-]*/uy,
    role: new RegExp('\\(\\s*' + NAME + '\\s*\\)', 'uy'),
    word: /[^\s{}();=\[\],<>"@#&%$?]+/y
  };

  function at(re, s, i) {
    re.lastIndex = i;
    var m = re.exec(s);
    return m ? m[0] : null;
  }

  function State() {
    this.inRec = false;
    this.depth = 0;
    this.opened = false;
    this.inCmt = false;
    this.paren = 0;
  }

  function hlRnl(s, st) {
    var out = '', i = 0, n = s.length, m;
    /* prev: what the last token was, when nothing separates it from the next character */
    var prev = '';
    while (i < n) {
      if (st.inCmt) {
        var e = s.indexOf('*/', i);
        if (e < 0) { out += span('t-cmt', s.slice(i)); return out; }
        out += span('t-cmt', s.slice(i, e + 2));
        i = e + 2;
        st.inCmt = false;
        continue;
      }
      if (!st.inRec) {
        var rest = s.slice(i);
        var k = rest.search(/@RNL\b/);
        if (k < 0) { out += span('t-cmt', rest); return out; }
        if (k > 0) out += span('t-cmt', rest.slice(0, k));
        i += k;
        out += span('t-rec', '@RNL');
        i += 4;
        st.inRec = true; st.depth = 0; st.opened = false; st.paren = 0;
        prev = 'kw';
        continue;
      }
      var c = s.charAt(i);
      var inParams = st.paren > 0;
      if (/\s/.test(c)) { m = at(RE.ws, s, i); out += m; i += m.length; prev = ''; continue; }
      if (c === '/' && s.charAt(i + 1) === '*') { st.inCmt = true; continue; }
      if (c === '(') {
        if (prev === 'kw') { st.paren++; out += span('t-punct', '('); i++; prev = ''; continue; }
        if (prev === 'ref' && !inParams && (m = at(RE.role, s, i))) { out += span('t-role', m); i += m.length; prev = ''; continue; }
        if (inParams) { st.paren++; out += span('t-punct', '('); i++; prev = ''; continue; }
        if ((m = at(RE.pt, s, i))) { out += span('t-pt', m); i += m.length; prev = 'ref'; continue; }
        out += span('t-free', '('); i++; prev = ''; continue;
      }
      if (c === ')') {
        if (st.paren > 0) { st.paren--; out += span('t-punct', ')'); } else out += span('t-free', ')');
        i++; prev = ''; continue;
      }
      if (c === '{') {
        if (inParams) { out += span('t-punct', '{'); i++; continue; }
        st.depth++; st.opened = true; out += span('t-punct', '{'); i++; prev = ''; continue;
      }
      if (c === '}') {
        if (inParams) { out += span('t-punct', '}'); i++; continue; }
        st.depth--; out += span('t-punct', '}'); i++; prev = 'ref';
        if (st.opened && st.depth <= 0) { st.inRec = false; st.depth = 0; }
        continue;
      }
      if (c === '"') { m = at(RE.str, s, i); out += span('t-str', m); i += m.length; prev = ''; continue; }
      if (c === '@' && (m = at(RE.kw, s, i))) {
        out += span(m === '@RNL' ? 't-rec' : 't-kw', m); i += m.length; prev = 'kw'; continue;
      }
      if (c === '#' && (m = at(RE.inst, s, i))) { out += span('t-inst', m); i += m.length; prev = 'ref'; continue; }
      if (c === '&' && (m = at(RE.net, s, i))) { out += span('t-net', m); i += m.length; prev = 'ref'; continue; }
      if (c === '%' && (m = at(RE.obj, s, i))) { out += span('t-obj', m); i += m.length; prev = 'ref'; continue; }
      if (c === '$' && (m = at(RE.def, s, i))) { out += span('t-def', m); i += m.length; prev = ''; continue; }
      if (c === '?') {
        m = at(RE.slot, s, i) || '?';
        out += span('t-slot', m); i += m.length; prev = 'ref'; continue;
      }
      if (c === '=' && s.charAt(i + 1) === '>') { out += span('t-punct t-pipe', '=>'); i += 2; prev = ''; continue; }
      if (c === ';' || c === '=') { out += span('t-punct', c); i++; prev = ''; continue; }
      if (c === '[' || c === ']' || c === ',') { out += span(inParams ? 't-punct' : 't-free', c); i++; prev = ''; continue; }
      if (inParams && /[-+.\d]/.test(c) && (m = at(RE.num, s, i))) {
        /* A number followed directly by a letter (such as 2O) is not a valid number: flag it */
        var glued = /[\p{L}_]/u.test(s.charAt(i + m.length));
        if (glued) {
          var tail = at(RE.ident, s, i + m.length) || '';
          out += span('t-bad', m + tail); i += m.length + tail.length;
        } else { out += span('t-num', m); i += m.length; }
        prev = ''; continue;
      }
      if (inParams && (m = at(RE.ident, s, i))) {
        var j = i + m.length;
        while (j < n && s.charAt(j) === ' ') j++;
        out += span(s.charAt(j) === '=' && s.charAt(j + 1) !== '>' ? 't-pn' : 't-lit', m);
        i += m.length; prev = ''; continue;
      }
      m = at(RE.word, s, i);
      if (m) { out += span('t-free', m); i += m.length; prev = ''; continue; }
      out += span('t-free', c); i++; prev = '';
    }
    return out;
  }

  /* Whole file: returns each line's HTML and kind */
  function hlFile(texts) {
    var st = new State();
    return texts.map(function (text) {
      var m = /^(\s*)(\*|\/\/)(.*)$/.exec(text);
      if (m && (st.inRec || st.inCmt || /@RNL\b/.test(m[3]))) {
        return { html: esc(m[1]) + span('t-pref', m[2]) + hlRnl(m[3], st), rnl: true };
      }
      if (m) return { html: esc(m[1]) + span('t-pref', m[2]) + span('t-cmt', m[3]), cmt: true };
      if (!text.trim()) return { html: '', blank: true };
      return { html: hlSpice(text), host: true };
    });
  }

  /* A fragment inside a record (without the host prefix), for inline examples */
  function hlSnippet(text) {
    var st = new State();
    st.inRec = true; st.opened = true; st.depth = 1;
    return text.split('\n').map(function (line) { return hlRnl(line, st); }).join('\n');
  }

  /* A whole file when any line starts with a host comment prefix or nothing in it looks like RNL
     (plain netlist lines); otherwise a fragment inside a record */
  function hlAuto(text) {
    if (/^\s*(\*|\/\/)/m.test(text) || !/[@#&%$]/.test(text)) return hlFile(text.split('\n')).map(function (h) { return h.html; }).join('\n');
    return hlSnippet(text);
  }

  /* ---------- Code panels ---------- */

  function render(container, lines) {
    var hl = hlFile(lines.map(function (l) { return l.t; }));
    var html = '';
    lines.forEach(function (l, i) {
      var h = hl[i];
      var cls = 'cl' + (h.rnl ? ' is-rnl' : '') + (l.r ? ' is-link' : '') + (l.cls ? ' ' + l.cls : '') + (l.mark ? ' ' + l.mark : '');
      html += '<div class="' + cls + '" data-i="' + i + '"' + (l.r ? ' data-refs="' + esc(l.r) + '" tabindex="0"' : '') + '>' +
        '<span class="ln" aria-hidden="true">' + (i + 1) + '</span><span class="tx">' + (h.html || ' ') + '</span></div>';
    });
    container.innerHTML = html;
    return Array.prototype.slice.call(container.querySelectorAll('.cl'));
  }

  /* Cross-highlighting between code lines and the drawing: hovering a line highlights the objects it references; hovering an object in the drawing marks the lines that reference it */
  function link(container, lines, view, opts) {
    opts = opts || {};
    var current = -1;
    function rows() { return Array.prototype.slice.call(container.querySelectorAll('.cl')); }
    function setLine(i) {
      if (i === current) return;
      current = i;
      rows().forEach(function (r) { r.classList.remove('is-hot'); });
      if (i < 0) {
        if (view) { view.highlight(null); if (opts.modes) view.setMode(null); }
        if (opts.onLine) opts.onLine(-1, null);
        return;
      }
      var l = lines[i];
      var row = rows()[i];
      if (row) row.classList.add('is-hot');
      if (view) {
        if (opts.modes) view.setMode(l.mode || null);
        view.highlight(l.r ? l.r.split(/\s+/) : null);
      }
      if (opts.onLine) opts.onLine(i, l);
    }
    function fromEvent(e) {
      var row = e.target.closest ? e.target.closest('.cl') : null;
      if (!row || !container.contains(row)) return -1;
      var i = +row.getAttribute('data-i');
      return lines[i] && (lines[i].r || opts.allLines) ? i : -1;
    }
    container.addEventListener('mouseover', function (e) { setLine(fromEvent(e)); });
    container.addEventListener('mouseleave', function () { setLine(-1); });
    container.addEventListener('focusin', function (e) { setLine(fromEvent(e)); });
    container.addEventListener('focusout', function () { setLine(-1); });
    return {
      setLine: setLine,
      setLines: function (ls) { lines = ls; current = -1; },
      /* Hovering an object in the drawing → mark the lines that reference it */
      markRef: function (ref) {
        rows().forEach(function (r) {
          var refs = (r.getAttribute('data-refs') || '').split(/\s+/);
          r.classList.toggle('is-hot', !!ref && refs.indexOf(ref) >= 0);
        });
        if (view) view.highlight(ref ? [ref] : null);
      }
    };
  }

  global.RNLCode = { hlFile: hlFile, hlSnippet: hlSnippet, hlAuto: hlAuto, render: render, link: link, esc: esc };
})(window);
