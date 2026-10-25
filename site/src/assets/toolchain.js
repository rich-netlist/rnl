/* RNL toolchain diagram · interactions of the toolchain section of the page of the tools */
(function () {
  'use strict';

  var S = window.RNLSchem, PG = window.RNLPage;
  var $ = PG.$, $$ = PG.$$, icon = PG.icon, debounce = PG.debounce;

  /* nm: module name; sub: small caption after the name, shown when it differs from nm */
  var MODS = {
    file: {
      nm: '<en>Netlist + RNL file</en>', sub: 'SPICE / Spectre',
      role: '<en>Where everything starts and ends.</en>',
      li: [
        '<en>The netlist defines the circuit; RNL in the comments describes how to understand and draw it</en>',
        '<en>Simulators read the whole file as usual</en>',
        '<en>The solver replaces the records with type="generated"; an editor changes only the records it wrote</en>'
      ],
      form: '<en>Existing netlist files such as .sp and .scs</en>',
      st: ['chip', '<en>Existing formats</en>']
    },
    adapter: {
      nm: '<en>Host adapters</en>', sub: 'Host adapters',
      role: '<en>Understands netlist dialects.</en>',
      li: [
        '<en>Finds the comment lines where RNL can be written safely</en>',
        '<en>Reads devices, nets, terminal names and the subcircuit hierarchy</en>',
        '<en>When only RNL changes, the netlist body and ordinary comments are kept as is</en>'
      ],
      form: '<en>rnl-host-spice, rnl-host-spectre (Rust)</en>',
      st: ['chip chip-ok', '<en>In 0.1</en>']
    },
    runtime: {
      nm: '<en>Runtime</en>', sub: 'Runtime',
      role: '<en>Understands RNL.</en>',
      li: [
        '<en>Parses the syntax, locating errors by line, field and reference</en>',
        '<en>Resolves names, scopes and downward inheritance; loads the standard and alias packages</en>',
        '<en>Takes effect in phases: symbols first, then geometry by priority, then styles</en>',
        '<en>Writes back to the file, preserving the original formatting</en>'
      ],
      form: '<en>rnl-runtime (Rust, compiles to WASM); npm package @rich-netlist/runtime</en>',
      st: ['chip chip-ok', '<en>In 0.1 · without solving</en>']
    },
    solver: {
      nm: '<en>Solver</en>', sub: 'Solver',
      role: '<en>Turns intent into coordinates.</en>',
      li: [
        '<en>Input: the effective model of one circuit level from the runtime, that is, semantics, constraints and priorities</en>',
        '<en>Output: complete positions, orientations and routing for that level, as one record with type="generated" and priority high</en>',
        '<en>Delivers better results as it computes, and can be cancelled at any time</en>',
        '<en>Same input and same work budget give exactly the same result</en>'
      ],
      form: '<en>rnl-solver (pure Rust, compiles to WASM); @rich-netlist/solver</en>',
      st: ['chip chip-warn', '<en>First version · CLI and desktop</en>']
    },
    render: {
      nm: '<en>Renderer</en>', sub: 'Renderer',
      role: '<en>Turns descriptions into graphics.</en>',
      li: [
        '<en>Expands symbols and applies orientations</en>',
        '<en>Outputs lines, fills and text that keep their object identity</en>',
        '<en>The viewer and the CLI draw exactly the same picture</en>'
      ],
      form: '<en>rnl-render (Rust, part of the shared core with the runtime)</en>',
      st: ['chip chip-ok', '<en>In 0.1</en>']
    },
    viewer: {
      nm: '<en>Viewer</en>', sub: 'Viewer',
      role: '<en>As light as a PDF reader.</en>',
      li: [
        '<en>Zoom, pan, search for devices</en>',
        '<en>Highlight a net, descend into subcircuits</en>',
        '<en>No solver needed when the file already has full geometry</en>',
        '<en>Edit the text and draw it again on Apply; a file is written only on Save, which on the web is a download</en>',
        '<en>No built-in component library; symbols come from the standard package and RNL libraries</en>'
      ],
      form: '<en>apps/web and apps/desktop, sharing the page apps/viewer-ui</en>',
      st: ['chip chip-ok', '<en>Since 0.1 · edits; desktop solves</en>']
    },
    editor: {
      nm: '<en>Editor</en>', sub: 'Editor',
      role: '<en>Editing a drawing appends a line.</en>',
      li: [
        '<en>Drag, swap symbols, restyle, with undo and redo</en>',
        '<en>Edits go into a record of the editor’s own, appended after the solver’s result, with priority high</en>',
        '<en>Built-in lightweight routing, so dragging never waits for the solver</en>'
      ],
      form: '@rich-netlist/editor',
      st: ['chip chip-warn', '<en>On hold</en>']
    },
    cli: {
      nm: '<en>CLI</en>', sub: 'CLI',
      role: '<en>Batch work and automation.</en>',
      li: [
        '<en>Checks syntax, references and the basic validity of each statement</en>',
        '<en>Solves semantics and intent into coordinates and writes them back</en>',
        '<en>Exports SVG or PDF</en>',
        '<en>Migrates old syntax versions to new ones</en>'
      ],
      form: '<en>rnl-cli (Rust, prebuilt for each platform)</en>',
      st: ['chip chip-ok', '<en>Since 0.1 · checks, renders, solves</en>']
    },
    stdlib: {
      nm: '<en>Standard library</en>', sub: 'Standard library',
      role: '<en>Symbols are data too.</en>',
      li: [
        '<en>The standard package: symbols, styles, semantic and layout definitions and fill rules, itself written in RNL</en>',
        '<en>Alias packages, such as the Chinese rnl-zh</en>',
        '<en>Released and versioned separately from the engines; a definition of the same name in a file overrides it</en>'
      ],
      form: '<en>@rich-netlist/stdlib (data-only package)</en>',
      st: ['chip chip-ok', '<en>Series 0.2 and 0.3</en>']
    }
  };

  var EDGES = [
    { id: 'f-a', from: 'file', to: 'adapter', route: 'h' },
    { id: 'a-r', from: 'adapter', to: 'runtime', route: 'h' },
    { id: 'r-ren', from: 'runtime', to: 'render', route: 'h' },
    { id: 'ren-v', from: 'render', to: 'viewer', route: 'h' },
    { id: 'r-s', from: 'runtime', to: 'solver', route: 'v', fa: 0.3 },
    { id: 's-r', from: 'solver', to: 'runtime', route: 'vu', fa: 0.7 },
    { id: 'ren-cli', from: 'render', to: 'cli', route: 'v', fa: 0.5, label: 'SVG / PDF', side: 'r' },
    { id: 'v-e', from: 'viewer', to: 'editor', route: 'v', fa: 0.5 },
    { id: 'e-r', from: 'editor', to: 'runtime', route: 'edit', label: '<en>append a record</en>' },
    { id: 'lib-r', from: 'stdlib', to: 'runtime', route: 'lib', label: '<en>standard and alias packages</en>' },
    { id: 'r-f', from: 'runtime', to: 'file', route: 'back', label: '<en>write back (text preserved)</en>' }
  ];

  var FLOWS = {
    open: { mods: ['file', 'adapter', 'runtime', 'render', 'viewer', 'stdlib'], edges: ['f-a', 'a-r', 'r-ren', 'ren-v', 'lib-r'], cap: '<en>The file already has full geometry: the viewer draws it directly, no solver needed.</en>', skip: ['solver'] },
    solve: { mods: ['file', 'adapter', 'runtime', 'solver', 'render', 'viewer', 'cli', 'stdlib'], edges: ['f-a', 'a-r', 'r-s', 's-r', 'r-ren', 'ren-v', 'ren-cli', 'lib-r', 'r-f'], cap: '<en>Only semantics and intent: the solver computes coordinates and routing and writes them back as a record with type="generated"; next time the file opens ready to view.</en>' },
    edit: { mods: ['editor', 'viewer', 'runtime', 'solver', 'render', 'file'], edges: ['v-e', 'e-r', 'r-s', 's-r', 'r-ren', 'ren-v', 'r-f'], cap: '<en>Drag a device in the editor: a record of the editor’s own (high) is appended, the solver re-solves in the background, and better results refresh onto the screen.</en>' }
  };

  function initTools() {
    var graph = $('#tgraph'), svg = $('#twires'), detail = $('#tdetail');
    if (!graph) return;
    var mods = {};
    $$('.tmod', graph).forEach(function (m) { mods[m.getAttribute('data-mod')] = m; });
    var flow = 'solve', sel = 'solver';

    function R(id) {
      var g = graph.getBoundingClientRect(), r = mods[id].getBoundingClientRect();
      var l = r.left - g.left, t = r.top - g.top;
      return { l: l, t: t, r: l + r.width, b: t + r.height, w: r.width, h: r.height, cx: l + r.width / 2, cy: t + r.height / 2 };
    }
    function pts(e) {
      var A = R(e.from), B = R(e.to), x, y;
      switch (e.route) {
        case 'h': return [[A.r + 4, A.cy], [B.l - 4, B.cy]];
        case 'v': x = A.l + A.w * e.fa; return [[x, A.b + 4], [x, B.t - 4]];
        case 'vu': x = A.l + A.w * e.fa; return [[x, A.t - 4], [x, B.b + 4]];
        case 'lib':
          y = R('file').b + (A.t - R('file').b) * 0.52;
          x = B.l + B.w * 0.12;
          return [[A.cx, A.t - 4], [A.cx, y], [x, y], [x, B.b + 4]];
        case 'edit':
          y = R('viewer').b + (A.t - R('viewer').b) * 0.78;
          var xs = A.l + A.w * 0.24, xe = B.l + B.w * 0.9;
          return [[xs, A.t - 4], [xs, y], [xe, y], [xe, B.b + 4]];
        case 'back':
          y = A.t - 22;
          return [[A.cx, A.t - 4], [A.cx, y], [B.cx, y], [B.cx, B.t - 4]];
      }
      return [];
    }
    function d(p) { return 'M' + p.map(function (q) { return q[0].toFixed(1) + ' ' + q[1].toFixed(1); }).join(' L'); }

    function draw() {
      while (svg.firstChild) svg.removeChild(svg.firstChild);
      if (window.getComputedStyle(svg).display === 'none') return;
      var defs = S.el('defs', null, svg);
      ['off', 'on'].forEach(function (k) {
        var mk = S.el('marker', { id: 'tm-' + k, viewBox: '0 0 10 10', refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: 'auto-start-reverse' }, defs);
        S.el('path', { d: 'M0 0L10 5L0 10z', class: 'tm-' + k }, mk);
      });
      var F = FLOWS[flow];
      EDGES.forEach(function (e) {
        var p = pts(e);
        var on = F.edges.indexOf(e.id) >= 0;
        S.el('path', { d: d(p), class: on ? 'is-on' : 'is-dim', 'marker-end': 'url(#tm-' + (on ? 'on' : 'off') + ')' }, svg);
        if (e.label) {
          var anchor = 'middle', lx, ly;
          if (p.length === 2) {
            /* Vertical edge: put the label in the upper part of the gap so it stays clear of the horizontal edges below */
            lx = p[0][0] + (e.side === 'l' ? -7 : 7);
            ly = Math.min(p[0][1], p[1][1]) + 16;
            anchor = e.side === 'l' ? 'end' : 'start';
          } else if (e.route === 'edit') {
            lx = p[0][0] - 8;
            ly = p[1][1] - 6;
            anchor = 'end';
          } else {
            lx = (p[1][0] + p[2][0]) / 2;
            ly = p[1][1] - 6;
          }
          var t = S.el('text', { x: lx.toFixed(1), y: ly.toFixed(1), 'text-anchor': anchor, class: on ? 'is-on' : '' }, svg);
          t.textContent = e.label;
        }
      });
    }

    function isFile(id) { return id === 'file'; }
    function paint() {
      var F = FLOWS[flow];
      Object.keys(mods).forEach(function (k) {
        mods[k].classList.toggle('is-on', F.mods.indexOf(k) >= 0);
        mods[k].classList.toggle('is-dim', F.mods.indexOf(k) < 0);
        mods[k].classList.toggle('is-sel', k === sel);
        mods[k].setAttribute('aria-pressed', k === sel ? 'true' : 'false');
        var tag = mods[k].querySelector('.skip-tag');
        if (F.skip && F.skip.indexOf(k) >= 0) {
          if (!tag) { tag = document.createElement('span'); tag.className = 'skip-tag'; tag.textContent = '<en>not needed</en>'; mods[k].appendChild(tag); }
        } else if (tag) tag.remove();
      });
      $('#flow-cap').textContent = F.cap;
      var m = MODS[sel];
      detail.innerHTML =
        '<h3>' + m.nm + (m.sub !== m.nm ? ' <small>' + m.sub + '</small>' : '') + '</h3>' +
        '<p class="role">' + m.role + '</p>' +
        '<ul>' + m.li.map(function (x) { return '<li>' + icon('check') + '<span>' + x + '</span></li>'; }).join('') + '</ul>' +
        '<dl class="kv"><dt><en>Form</en></dt><dd>' + m.form +
        (isFile(sel) ? '' : '<br><span class="muted"><en>All names are tentative</en></span>') +
        '</dd><dt><en>Status</en></dt><dd><span class="' + m.st[0] + '">' + m.st[1] + '</span></dd></dl>';
      draw();
    }

    PG.initSeg($('#flow-seg'), 'data-flow', function (v) { flow = v; paint(); });
    Object.keys(mods).forEach(function (k) {
      mods[k].addEventListener('click', function () { sel = k; paint(); });
    });
    paint();
    window.addEventListener('resize', debounce(draw, 150));
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(draw);
  }

  function boot() {
    initTools();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();
