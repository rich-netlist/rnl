/* RNL toolchain diagram · interactions of the toolchain section of the page of the tools */
(function () {
  'use strict';

  var S = window.RNLSchem, PG = window.RNLPage;
  var $ = PG.$, $$ = PG.$$, icon = PG.icon, debounce = PG.debounce;

  /* nm: module name; sub: small caption after the name, shown when it differs from nm */
  var MODS = {
    file: {
      nm: '<zh>网表 + RNL 文件</zh><en>Netlist + RNL file</en>', sub: 'SPICE / Spectre',
      role: '<zh>一切的起点和终点。</zh><en>Where everything starts and ends.</en>',
      li: [
        '<zh>电路由网表定义，理解与画法由注释里的 RNL 描述</zh><en>The netlist defines the circuit; RNL in the comments describes how to understand and draw it</en>',
        '<zh>仿真器照常读取整份文件</zh><en>Simulators read the whole file as usual</en>',
        '<zh>求解器换掉 type="generated" 的记录，编辑器只改自己写的记录</zh><en>The solver replaces the records with type="generated"; an editor changes only the records it wrote</en>'
      ],
      form: '<zh>.sp、.scs 等原有网表文件</zh><en>Existing netlist files such as .sp and .scs</en>',
      st: ['chip', '<zh>沿用现有格式</zh><en>Existing formats</en>']
    },
    adapter: {
      nm: '<zh>宿主适配器</zh><en>Host adapters</en>', sub: 'Host adapters',
      role: '<zh>读懂各种网表方言。</zh><en>Understands netlist dialects.</en>',
      li: [
        '<zh>找出可以安全写入 RNL 的注释行</zh><en>Finds the comment lines where RNL can be written safely</en>',
        '<zh>读出器件、网络、端子名和子电路层次</zh><en>Reads devices, nets, terminal names and the subcircuit hierarchy</en>',
        '<zh>只改 RNL 时，网表正文和普通注释原样保留</zh><en>When only RNL changes, the netlist body and ordinary comments are kept as is</en>'
      ],
      form: '<zh>rnl-host-spice、rnl-host-spectre（Rust）</zh><en>rnl-host-spice, rnl-host-spectre (Rust)</en>',
      st: ['chip chip-ok', '<zh>0.1 已有</zh><en>In 0.1</en>']
    },
    runtime: {
      nm: '<zh>运行库</zh><en>Runtime</en>', sub: 'Runtime',
      role: '<zh>读懂 RNL。</zh><en>Understands RNL.</en>',
      li: [
        '<zh>解析语法，错误定位到行、字段和引用</zh><en>Parses the syntax, locating errors by line, field and reference</en>',
        '<zh>解析名字、作用域和向下继承；加载标准包和代称包</zh><en>Resolves names, scopes and downward inheritance; loads the standard and alias packages</en>',
        '<zh>按阶段生效：先定符号，再按优先级合成几何，最后定样式</zh><en>Takes effect in phases: symbols first, then geometry by priority, then styles</en>',
        '<zh>保留原文格式写回文件</zh><en>Writes back to the file, preserving the original formatting</en>'
      ],
      form: '<zh>rnl-runtime（Rust，可编译到 WASM）；npm 包 @rich-netlist/runtime</zh><en>rnl-runtime (Rust, compiles to WASM); npm package @rich-netlist/runtime</en>',
      st: ['chip chip-ok', '<zh>0.1 已有 · 不含求解</zh><en>In 0.1 · without solving</en>']
    },
    solver: {
      nm: '<zh>求解器</zh><en>Solver</en>', sub: 'Solver',
      role: '<zh>把意图变成坐标。</zh><en>Turns intent into coordinates.</en>',
      li: [
        '<zh>输入：运行库交来的一层电路的生效模型，即语义、约束和优先级</zh><en>Input: the effective model of one circuit level from the runtime, that is, semantics, constraints and priorities</en>',
        '<zh>输出：这一层完整的位置、朝向和走线，写成一条 type="generated"、优先级 high 的记录</zh><en>Output: complete positions, orientations and routing for that level, as one record with type="generated" and priority high</en>',
        '<zh>边算边交付更好的结果，可以随时取消</zh><en>Delivers better results as it computes, and can be cancelled at any time</en>',
        '<zh>同一输入、同一工作量预算，结果完全相同</zh><en>Same input and same work budget give exactly the same result</en>'
      ],
      form: '<zh>rnl-solver（纯 Rust，可编译到 WASM）；@rich-netlist/solver</zh><en>rnl-solver (pure Rust, compiles to WASM); @rich-netlist/solver</en>',
      st: ['chip chip-warn', '<zh>第一版 · 命令行和桌面版</zh><en>First version · CLI and desktop</en>']
    },
    render: {
      nm: '<zh>统一渲染器</zh><en>Renderer</en>', sub: 'Renderer',
      role: '<zh>把描述变成图元。</zh><en>Turns descriptions into graphics.</en>',
      li: [
        '<zh>展开符号，应用朝向</zh><en>Expands symbols and applies orientations</en>',
        '<zh>输出带对象身份的线条、填充和文字</zh><en>Outputs lines, fills and text that keep their object identity</en>',
        '<zh>查看器和命令行画出来完全一致</zh><en>The viewer and the CLI draw exactly the same picture</en>'
      ],
      form: '<zh>rnl-render（Rust，与运行库同属共享核心）</zh><en>rnl-render (Rust, part of the shared core with the runtime)</en>',
      st: ['chip chip-ok', '<zh>0.1 已有</zh><en>In 0.1</en>']
    },
    viewer: {
      nm: '<zh>查看器</zh><en>Viewer</en>', sub: 'Viewer',
      role: '<zh>像 PDF 阅读器一样轻。</zh><en>As light as a PDF reader.</en>',
      li: [
        '<zh>缩放、平移、搜索器件</zh><en>Zoom, pan, search for devices</en>',
        '<zh>高亮一条网络，进入子电路</zh><en>Highlight a net, descend into subcircuits</en>',
        '<zh>文件里已有完整几何时，不需要求解器</zh><en>No solver needed when the file already has full geometry</en>',
        '<zh>改文本，应用后重画；保存时才写文件，网页版是下载</zh><en>Edit the text and draw it again on Apply; a file is written only on Save, which on the web is a download</en>',
        '<zh>不自带元件库，符号来自标准包和 RNL 库</zh><en>No built-in component library; symbols come from the standard package and RNL libraries</en>'
      ],
      form: '<zh>apps/web、apps/desktop，共用页面 apps/viewer-ui</zh><en>apps/web and apps/desktop, sharing the page apps/viewer-ui</en>',
      st: ['chip chip-ok', '<zh>0.1 起已有 · 可编辑，桌面版可求解</zh><en>Since 0.1 · edits; desktop solves</en>']
    },
    editor: {
      nm: '<zh>编辑器</zh><en>Editor</en>', sub: 'Editor',
      role: '<zh>改图就是追加一行。</zh><en>Editing a drawing appends a line.</en>',
      li: [
        '<zh>拖动、换符号、改样式，支持撤销重做</zh><en>Drag, swap symbols, restyle, with undo and redo</en>',
        '<zh>修改写成编辑器自己的记录，追加在求解结果之后，优先级 high</zh><en>Edits go into a record of the editor’s own, appended after the solver’s result, with priority high</en>',
        '<zh>内置轻量走线，拖动时不必等求解器</zh><en>Built-in lightweight routing, so dragging never waits for the solver</en>'
      ],
      form: '@rich-netlist/editor',
      st: ['chip chip-warn', '<zh>暂缓</zh><en>On hold</en>']
    },
    cli: {
      nm: '<zh>命令行</zh><en>CLI</en>', sub: 'CLI',
      role: '<zh>批量处理和自动化。</zh><en>Batch work and automation.</en>',
      li: [
        '<zh>检查语法、引用和单条描述的基本合法性</zh><en>Checks syntax, references and the basic validity of each statement</en>',
        '<zh>把语义和意图求解成坐标并写回</zh><en>Solves semantics and intent into coordinates and writes them back</en>',
        '<zh>导出 SVG 或 PDF</zh><en>Exports SVG or PDF</en>',
        '<zh>把旧版本语法迁移到新版本</zh><en>Migrates old syntax versions to new ones</en>'
      ],
      form: '<zh>rnl-cli（Rust，各平台预编译）</zh><en>rnl-cli (Rust, prebuilt for each platform)</en>',
      st: ['chip chip-ok', '<zh>0.1 起已有 · 检查、出图、求解</zh><en>Since 0.1 · checks, renders, solves</en>']
    },
    stdlib: {
      nm: '<zh>标准库</zh><en>Standard library</en>', sub: 'Standard library',
      role: '<zh>符号也是数据。</zh><en>Symbols are data too.</en>',
      li: [
        '<zh>标准包：符号、样式、语义和布局定义、补位规则，本身用 RNL 写成</zh><en>The standard package: symbols, styles, semantic and layout definitions and fill rules, itself written in RNL</en>',
        '<zh>代称包，如中文的 rnl-zh</zh><en>Alias packages, such as the Chinese rnl-zh</en>',
        '<zh>和引擎分开发布、分开编版本号；文件里写的同名定义盖过它</zh><en>Released and versioned separately from the engines; a definition of the same name in a file overrides it</en>'
      ],
      form: '<zh>@rich-netlist/stdlib（纯数据包）</zh><en>@rich-netlist/stdlib (data-only package)</en>',
      st: ['chip chip-ok', '<zh>标准系列 0.2 和 0.3</zh><en>Series 0.2 and 0.3</en>']
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
    { id: 'e-r', from: 'editor', to: 'runtime', route: 'edit', label: '<zh>追加编辑器的记录</zh><en>append a record</en>' },
    { id: 'lib-r', from: 'stdlib', to: 'runtime', route: 'lib', label: '<zh>标准包、代称包</zh><en>standard and alias packages</en>' },
    { id: 'r-f', from: 'runtime', to: 'file', route: 'back', label: '<zh>写回（保留原文）</zh><en>write back (text preserved)</en>' }
  ];

  var FLOWS = {
    open: { mods: ['file', 'adapter', 'runtime', 'render', 'viewer', 'stdlib'], edges: ['f-a', 'a-r', 'r-ren', 'ren-v', 'lib-r'], cap: '<zh>文件里已经有完整的几何：查看器直接画出来，不需要求解器。</zh><en>The file already has full geometry: the viewer draws it directly, no solver needed.</en>', skip: ['solver'] },
    solve: { mods: ['file', 'adapter', 'runtime', 'solver', 'render', 'viewer', 'cli', 'stdlib'], edges: ['f-a', 'a-r', 'r-s', 's-r', 'r-ren', 'ren-v', 'ren-cli', 'lib-r', 'r-f'], cap: '<zh>只有语义和意图：求解器算出坐标和走线，写回成一条 type="generated" 的记录；下次打开直接看。</zh><en>Only semantics and intent: the solver computes coordinates and routing and writes them back as a record with type="generated"; next time the file opens ready to view.</en>' },
    edit: { mods: ['editor', 'viewer', 'runtime', 'solver', 'render', 'file'], edges: ['v-e', 'e-r', 'r-s', 's-r', 'r-ren', 'ren-v', 'r-f'], cap: '<zh>在编辑器里拖动器件：追加一条编辑器的记录（high），后台重新求解，更好的结果陆续刷新到画面上。</zh><en>Drag a device in the editor: a record of the editor’s own (high) is appended, the solver re-solves in the background, and better results refresh onto the screen.</en>' }
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
          if (!tag) { tag = document.createElement('span'); tag.className = 'skip-tag'; tag.textContent = '<zh>不需要</zh><en>not needed</en>'; mods[k].appendChild(tag); }
        } else if (tag) tag.remove();
      });
      $('#flow-cap').textContent = F.cap;
      var m = MODS[sel];
      detail.innerHTML =
        '<h3>' + m.nm + (m.sub !== m.nm ? ' <small>' + m.sub + '</small>' : '') + '</h3>' +
        '<p class="role">' + m.role + '</p>' +
        '<ul>' + m.li.map(function (x) { return '<li>' + icon('check') + '<span>' + x + '</span></li>'; }).join('') + '</ul>' +
        '<dl class="kv"><dt><zh>形态</zh><en>Form</en></dt><dd>' + m.form +
        (isFile(sel) ? '' : '<br><span class="muted"><zh>名称均为暂定</zh><en>All names are tentative</en></span>') +
        '</dd><dt><zh>状态</zh><en>Status</en></dt><dd><span class="' + m.st[0] + '">' + m.st[1] + '</span></dd></dl>';
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
