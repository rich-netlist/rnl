/* RNL standard packages · the gallery
 *
 * Shows the data site/gallery/build.mjs writes into gallery/ at the root of the built site: index.js,
 * the groups of packages (a directory of package files each, such as the standard series 0.2), and a
 * file per group with every package, its statements and the symbols as the core drew them. They are
 * scripts, loaded with <script>, so that the page works opened from the disk as well.
 * The data is the same for both languages; only the words around it are translated.
 */
(function (global) {
  'use strict';

  var P = global.RNLPage, C = global.RNLCode;
  var $ = P.$, $$ = P.$$, esc = C.esc, fill = P.fill;

  /* Where the data is, from a page of either language */
  var DATA = '../gallery/';
  var REPO = 'https://github.com/rich-netlist/rnl/blob/main/';

  /* What each package is for, in both languages; a package without a line here shows the comment
     its file starts with. A line holds for every version of the package, unless one is written for
     its name and version, such as 'rnl-std-fill 0.3', where the versions differ. */
  var SUMMARY = {
    'rnl-std': '<zh>标准包。每份网表的根上默认就用它；它只引用别的包，自己没有内容。</zh><en>The standard package, used at the root of every netlist by default. It only references other packages and has no content of its own.</en>',
    'rnl-std-core': '<zh>最小的组合：样式、符号和配符号的规则，把网表画出来，别的什么都不加——不补地、电源和端口标签。</zh><en>The smallest collection: styles, symbols and the rules that give them, enough to draw the netlist and nothing more — no grounds, supplies or port tags filled in.</en>',
    'rnl-std-style': '<zh>样式 $base、$bus、$signal、$note、$grey，和给每类对象默认样式的规则。</zh><en>The styles $base, $bus, $signal, $note and $grey, and the rules that give every kind of object its default style.</en>',
    'rnl-std-symbols': '<zh>元件的符号，以及按元件字母、Spectre 的 master 和模型类型给元件配符号的规则。</zh><en>The symbols of instances, and the rules that give an instance its symbol by element letter, by Spectre master and by model type.</en>',
    'rnl-std-virtual': '<zh>虚拟元件——地、电源、端口、不连接标记、连接点、电流箭头——它们的符号和配符号的规则，以及补地、补电源用的 stub。</zh><en>The virtual elements — ground, supply, port, no-connect mark, junction, current arrow — their symbols and the rules that give them, and the ground and supply stubs.</en>',
    'rnl-std-basic': '<zh>无源元件、电源、开关、传输线、虚拟元件和标记的符号；虚拟元件和纯画图元件的定义，补地、补电源的 stub，给它们配符号的规则。</zh><en>The symbols of passive parts, sources, switches, transmission lines, virtual elements and marks; the definitions of the virtual and drawing-only parts, the ground and supply stubs, and the rules that draw them.</en>',
    'rnl-std-semiconductors': '<zh>二极管、MOS、BJT 和 JFET 的符号。</zh><en>The symbols of diodes, MOS transistors, BJTs and JFETs.</en>',
    'rnl-std-spice': '<zh>SPICE 网表的配符号规则：先按元件字母，再按模型类型。</zh><en>The symbol rules of SPICE netlists: by element letter first, then by model type.</en>',
    'rnl-std-spectre': '<zh>Spectre 网表的配符号规则：内置元件按 master，模型卡的实例按模型类型。</zh><en>The symbol rules of Spectre netlists: primitives by master, instances of model cards by model type.</en>',
    'rnl-std-fill': '<zh>自动补全：给没连线的引脚补地或电源，给每个端口网络补端口标签；还有文件可以增删的集合。</zh><en>The automatic fill: a ground or supply for every unwired pin on such a net, a port tag for every port net; and the sets a file edits.</en>',
    'rnl-std-fill 0.3': '<zh>自动补全：给没连线的引脚补地或电源；给端口网络补端口标签，地和电源网络除外，那里已经有地和电源；不叫 0 的地网络（比如 vss）上的地，下面写上网络名。还有文件可以增删的集合。</zh><en>The automatic fill: a ground or supply for every unwired pin on such a net; a port tag for every port net but the ground and supply nets, which have their grounds and supplies already; and the grounds of a ground net other than 0, such as vss, drawn with the name of the net under them. And the sets a file edits.</en>',
    'rnl-std-semantics': '<zh>语义定义，比如差分对、电流镜：只有角色，什么都不画。</zh><en>Semantic definitions, such as a differential pair or a current mirror: roles only, nothing drawn.</en>',
    'rnl-std-layout': '<zh>布局定义：镜像对、居中的尾管、信号流的方向。</zh><en>Layout definitions: a mirrored pair, a tail centred under a pair, the direction of signal flow.</en>',
    'rnl-std-structures': '<zh>结构定义：差分对、电流镜、支路、链条、级联、镜像对称、信号流向等。有的自带默认画法，约束都是 low：作者写的优先，用例的 PRIORITY 能关掉。</zh><en>Structures: differential pair, current mirror, branch, chain, cascade, mirrored, flow and more. Some carry a default drawing, all of priority low: what the author writes comes first, and PRIORITY on a usage turns it off.</en>',
    'rnl-std-full': '<zh>rnl-std 再加上四个领域包：functional、rf、logic、signal-flow。</zh><en>rnl-std with the four domain packages: functional, rf, logic and signal-flow.</en>',
    'rnl-std-functional': '<zh>功能块：运放、比较器、放大器、跨导、ADC、DAC。</zh><en>Function blocks: op-amps, comparators, amplifiers, transconductors, ADC, DAC.</en>',
    'rnl-std-rf': '<zh>射频元件：中心抽头电感、T-coil。</zh><en>RF parts: the centre-tapped inductor and the T-coil.</en>',
    'rnl-std-logic': '<zh>逻辑门、缓冲器、反相器、D 触发器，和只画图的延迟单元。</zh><en>Logic gates, buffer, inverter, D flip-flops, and the drawing-only delay cell.</en>',
    'rnl-std-signal-flow': '<zh>信号流图：加法器、乘法器，和只画图的积分器、离散时间积分器、量化器、单位延迟。</zh><en>Signal-flow blocks: adder and multiplier, and the drawing-only integrator, discrete-time integrator, quantizer and unit delay.</en>',
    'rnl-std-no-labels': '<zh>关掉标准符号的默认标签，给自己写全部标签的文件用。</zh><en>Turns off the default labels of the standard symbols, for files that write all their labels themselves.</en>',
    'rnl-zh': '<zh>中文别名包：标准词和标准包名字的中文写法，在头段记录里用 @USE-ALIASES 载入。</zh><en>The Chinese alias package: Chinese spellings of the standard words and of the standard package’s names, loaded with @USE-ALIASES in the header record.</en>',
  };

  var DIRS = {
    0: '<zh>右</zh><en>right</en>', 90: '<zh>上</zh><en>up</en>', 180: '<zh>左</zh><en>left</en>', 270: '<zh>下</zh><en>down</en>'
  };
  var ARROWS = { 0: '→', 90: '↑', 180: '←', 270: '↓' };

  var state = { index: null, groups: {}, group: null, pkg: null, q: '', expanded: {}, show: { pins: true, box: false, labels: true, origin: false } };
  var main, status;

  /* ---------------- loading ---------------- */

  global.RNLGallery = {
    index: function (data) { state.index = data; },
    group: function (data) { state.groups[data.id] = data; }
  };

  function load(src, done, fail) {
    var s = document.createElement('script');
    s.src = DATA + src;
    s.onload = done;
    s.onerror = fail;
    document.head.appendChild(s);
  }

  function groupTitle(g) {
    if (g.series) return fill('<zh>标准系列 {v}</zh><en>Standard series {v}</en>', { v: g.series });
    return g.packages.map(function (p) { return p.name + ' ' + p.version; }).join(', ');
  }

  /* ---------------- small pieces ---------------- */

  function code(s) { return '<code class="gl-code">' + C.hlSnippet(s) + '</code>'; }
  function name(n) { return '<code class="gl-name">' + breakable(n) + '</code>'; }
  /* A long name may break after its underscores and hyphens */
  function breakable(n) { return esc(n).replace(/([_-])/g, '$1<wbr>'); }
  function lineLink(pkg, line, end) {
    var href = (pkg.source || REPO + pkg.path) + '#L' + line + (end && end !== line ? '-L' + end : '');
    return '<a class="gl-line" href="' + href + '" title="<zh>在 GitHub 上看这几行</zh><en>See these lines on GitHub</en>">' +
      (end && end !== line ? fill('<zh>第 {a}–{b} 行</zh><en>lines {a}–{b}</en>', { a: line, b: end }) : fill('<zh>第 {a} 行</zh><en>line {a}</en>', { a: line })) + '</a>';
  }
  function docText(s) { return s.docs && s.docs.length ? s.docs[s.docs.length - 1] : ''; }
  function terminalText(p) { return [].concat(p.terminal).join(', '); }

  /* Every symbol of the shown group by name, with its package */
  function symbolIndex(group) {
    var map = {};
    group.packages.forEach(function (pkg) {
      pkg.statements.forEach(function (s) { if (s.kind === 'symbol' && s.svg) map[s.name] = { sym: s, pkg: pkg }; });
    });
    return map;
  }

  /* The rule that gives the usages of a definition their symbol: `# @SELECT-USAGE $GROUND => @SET-SYMBOL $GROUND_SYMBOL` in 0.3 */
  function drawnAs(group, def) {
    var re = new RegExp('@SELECT-USAGE\\s+\\' + def + '(?![\\w])');
    var found = null;
    group.packages.forEach(function (p) {
      p.statements.forEach(function (r) { if (r.kind === 'rule' && r.action === 'symbol' && re.test(r.select)) found = r; });
    });
    return found;
  }

  /* ---------------- search ---------------- */

  function terms() { return state.q.toLowerCase().split(/\s+/).filter(Boolean); }
  function matches(text) {
    var t = terms();
    if (!t.length) return true;
    var s = String(text).toLowerCase();
    return t.every(function (w) { return s.indexOf(w) >= 0 || s.indexOf(w.replace(/^[$%#]/, '')) >= 0; });
  }
  function statementText(s) {
    return [s.name || '', s.text || '', docText(s)].join(' ');
  }

  /* ---------------- rendering a package ---------------- */

  /* Append last so the decorative registration mark sits above every drawing and overlay. */
  function symbolSvg(s) {
    if (!s.origin) return s.svg;
    return s.svg.replace('</svg>', '<g class="g-origin" aria-hidden="true" transform="translate(' + s.origin.join(' ') + ')">' +
      '<g class="g-origin-mark"><line x1="-5" y1="0" x2="5" y2="0" vector-effect="non-scaling-stroke"/>' +
      '<line x1="0" y1="-5" x2="0" y2="5" vector-effect="non-scaling-stroke"/>' +
      '<circle r="1.5"/></g></g></svg>');
  }

  /* A card: the picture, whose pins carry their terminal numbers and directions, the name, and the
     names of the terminals when the package gives them, which the picture does not show */
  function symbolCard(pkg, s) {
    var pins = s.pins || [];
    var named = pins.some(function (p) { return p.label || p.name; });
    var row = !pins.length ? '<span class="gl-none"><zh>没有引脚</zh><en>no pins</en></span>'
      : named ? pins.map(function (p) {
        return '<span class="gl-pin">' + esc(terminalText(p)) + '<i>' + esc(p.label || p.name || '') + '</i></span>';
      }).join('') : '';
    var warn = s.problems && s.problems.length ? '<svg class="ico gl-warn" aria-label="<zh>构建时有提示</zh><en>The build noted something</en>"><use href="#i-warn"/></svg>' : '';
    var label = fill('<zh>{s}，{n} 个引脚</zh><en>{s}, {n} pins</en>', { s: s.name, n: pins.length }) + pins.map(function (p) {
      return fill('<zh>；端子 {t}{m}朝{d}</zh><en>; terminal {t}{m} pointing {d}</en>', { t: terminalText(p), m: p.label || p.name ? ' ' + (p.label || p.name) + ' ' : ' ', d: DIRS[p.dir] || '?' });
    }).join('');
    return '<button type="button" class="gl-card" data-sym="' + esc(s.name) + '" data-pkg="' + esc(pkg.name) + '" aria-label="' + esc(label) + '">' +
      '<span class="gl-fig" aria-hidden="true">' + symbolSvg(s) + '</span>' +
      '<span class="gl-card-name"><span>' + breakable(s.name) + '</span>' + warn + '<small class="gl-fit" hidden></small></span>' +
      (row ? '<span class="gl-pins">' + row + '</span>' : '') +
      '</button>';
  }

  /* A small picture of a symbol, framed on the symbol alone, that opens its details */
  function thumb(symName, entry) {
    var svg = symbolSvg(entry.sym);
    if (entry.sym.thumb) svg = svg.replace(/viewBox="[^"]*"/, 'viewBox="' + entry.sym.thumb + '"');
    return '<button type="button" class="gl-thumb" data-sym="' + esc(symName) + '" data-pkg="' + esc(entry.pkg.name) + '" aria-label="' + esc(symName) + '">' + svg + '</button>';
  }

  function ruleRows(pkg, rules, symbols) {
    return rules.map(function (r) {
      var target = r.target && symbols[r.target];
      return '<tr><td>' + code(r.select) + '</td><td><div class="gl-to">' + (target ? thumb(r.target, target) : '') + (r.target ? name(r.target) : code(r.last)) + '</div></td><td>' + lineLink(pkg, r.line) + '</td></tr>';
    }).join('');
  }

  function block(id, title, count, body) {
    return '<div class="gl-block" data-block="' + id + '"><h4>' + title + ' <span class="gl-count">' + count + '</span></h4>' + body + '</div>';
  }

  function renderPackage(group, pkg, symbols) {
    var st = pkg.statements;
    var syms = st.filter(function (s) { return s.kind === 'symbol' && s.svg && matches(statementText(s)); });
    var parts = st.filter(function (s) { return s.kind === 'symbol' && !s.svg && matches(statementText(s)); });
    var defs = st.filter(function (s) { return s.kind === 'definition' && matches(statementText(s)); });
    var styles = st.filter(function (s) { return s.kind === 'style' && matches(statementText(s)); });
    var sets = st.filter(function (s) { return (s.kind === 'set' || s.kind === 'apply' || (s.kind === 'rule' && /^(remove-from|add-to)$/.test(s.action))) && matches(statementText(s)); });
    var symRules = st.filter(function (s) { return s.kind === 'rule' && s.action === 'symbol' && matches(statementText(s)); });
    var otherRules = st.filter(function (s) { return s.kind === 'rule' && s.action !== 'symbol' && !/^(remove-from|add-to)$/.test(s.action) && matches(statementText(s)); });
    var aliases = (pkg.aliases || []).filter(function (a) { return matches(a.join(' ')); });
    var nameHit = terms().length && matches(pkg.name);
    var found = syms.length + parts.length + defs.length + styles.length + sets.length + symRules.length + otherRules.length + aliases.length;
    if (terms().length && !found && !nameHit) return { html: '', count: 0 };

    var h = '<section class="gl-pkg" id="' + esc(pkg.name) + '">';
    h += '<header class="gl-pkg-head"><div class="gl-pkg-title"><h3>' + esc(pkg.name) + '</h3>' +
      '<span class="chip">' + esc(pkg.version) + '</span>' +
      (pkg.implicit ? '<span class="chip chip-ok" title="<zh>rnl-std 带进来的包，每份网表默认就有</zh><en>Brought in by rnl-std: every netlist has it by default</en>"><zh>默认生效</zh><en>by default</en></span>'
        : '<span class="chip chip-user" title="<zh>文件在头段记录里用 USE-PACKAGE 才有</zh><en>A file has it when its header record names it with USE-PACKAGE</en>"><zh>按需使用</zh><en>on demand</en></span>') +
      (pkg.language !== pkg.version ? '<span class="chip chip-mute">' + fill('<zh>语言 {v}</zh><en>language {v}</en>', { v: esc(pkg.language) }) + '</span>' : '') +
      (group.series ? '' : '<span class="chip chip-warn"><zh>不属于标准系列</zh><en>not in the standard series</en></span>') +
      '</div>';
    var hasSummary = pkg.summary && (pkg.summary.en !== pkg.description.split('\n\n')[0] || pkg.summary.zh !== pkg.description.split('\n\n')[0]);
    var sum = hasSummary ? '<zh>' + esc(pkg.summary.zh) + '</zh><en>' + esc(pkg.summary.en) + '</en>' : SUMMARY[pkg.name + ' ' + pkg.version] || SUMMARY[pkg.name];
    h += '<p class="gl-sum">' + (sum || esc(pkg.description.split('\n\n')[0])) + '</p>';
    h += '<dl class="gl-meta">';
    var sourcePath = pkg.source ? new URL(pkg.source).pathname.replace(/^\/[^/]+\/[^/]+\/blob\/[^/]+\//, '') : pkg.path;
    h += '<div><dt><zh>文件</zh><en>File</en></dt><dd><a href="' + esc(pkg.source || REPO + pkg.path) + '">' + esc(sourcePath) + '</a></dd></div>';
    if (pkg.references.length) {
      h += '<div><dt><zh>引用</zh><en>References</en></dt><dd>' + pkg.references.map(function (r) {
        return '<a class="gl-ref" href="#' + esc(r.name) + '" data-goto="' + esc(r.name) + '">' + esc(r.name) + '</a>';
      }).join(' ') + '</dd></div>';
    }
    if (pkg.referencedBy.length) {
      h += '<div><dt><zh>被引用</zh><en>Referenced by</en></dt><dd>' + pkg.referencedBy.map(function (n) {
        return '<a class="gl-ref" href="#' + esc(n) + '" data-goto="' + esc(n) + '">' + esc(n) + '</a>';
      }).join(' ') + '</dd></div>';
    }
    if (pkg.expansion.length > 2) {
      h += '<div><dt><zh>展开顺序</zh><en>Brings in</en></dt><dd><ol class="gl-exp">' + pkg.expansion.map(function (n) {
        return '<li><a class="gl-ref" href="#' + esc(n) + '" data-goto="' + esc(n) + '">' + esc(n) + '</a></li>';
      }).join('') + '</ol></dd></div>';
    }
    h += '</dl>';
    if (pkg.problems && pkg.problems.length) {
      h += '<p class="gl-problem"><svg class="ico"><use href="#i-warn"/></svg><span>' + pkg.problems.map(function (p) {
        return lineLink(pkg, p.line) + ' ' + esc(p.text);
      }).join('<br>') + '</span></p>';
    }
    if (sum && pkg.description) {
      h += '<details class="gl-note"><summary><zh>文件开头的说明（英文）</zh><en>The comment the file starts with</en></summary><p>' + esc(pkg.description) + '</p></details>';
    }
    h += '</header>';

    if (syms.length) {
      h += block('symbols', '<zh>符号</zh><en>Symbols</en>', syms.length,
        '<div class="gl-grid">' + syms.map(function (s) { return symbolCard(pkg, s); }).join('') + '</div>');
    }
    if (symRules.length) {
      h += block('rules', '<zh>配符号的规则</zh><en>Symbol rules</en>', symRules.length,
        '<p class="gl-hint"><zh>按顺序执行，后面的覆盖前面的：先宽（元件字母），后窄（模型类型）。</zh><en>Applied in order, later ones overriding earlier ones: broad (element letter) first, narrow (model type) later.</en></p>' +
        '<div class="gl-table-wrap"><table class="gl-table gl-rules-table"><thead><tr><th><zh>选出</zh><en>Selects</en></th><th><zh>配上的符号</zh><en>Symbol</en></th><th></th></tr></thead><tbody>' +
        ruleRows(pkg, symRules, symbols) + '</tbody></table></div>');
    }
    if (defs.length) {
      h += block('definitions', '<zh>定义</zh><en>Definitions</en>', defs.length, '<ul class="gl-list">' + defs.map(function (s) {
        var drawn = drawnAs(group, s.name);
        var by = drawn && symbols[drawn.target];
        return '<li><div class="gl-li-head">' + name(s.name) + (s.roles ? code('roles=' + s.roles) : '') + (s.parameters ? code('parameters=' + s.parameters) : '') +
          (by ? '<span class="gl-drawn"><zh>画成</zh><en>drawn as</en> ' + thumb(drawn.target, by) + name(drawn.target) + '</span>' : '') +
          lineLink(pkg, s.line, s.endLine) + '</div>' +
          (docText(s) ? '<p class="gl-doc">' + esc(docText(s)) + '</p>' : '') +
          (s.body ? '<pre class="gl-pre">' + C.hlSnippet(s.text) + '</pre>' : '') + '</li>';
      }).join('') + '</ul>');
    }
    if (styles.length) {
      h += block('styles', '<zh>样式</zh><en>Styles</en>', styles.length, '<ul class="gl-list gl-styles">' + styles.map(function (s) {
        var props = Object.keys(s.props).map(function (k) {
          var v = s.props[k];
          var sw = /^#[0-9a-f]{3,8}$/i.test(v) ? '<span class="gl-sw" style="background:' + esc(v) + '"></span>' : '';
          return '<span class="gl-prop">' + sw + esc(k) + '=<b>' + esc(v) + '</b></span>';
        }).join('');
        return '<li><div class="gl-li-head">' + name(s.name) + (s.base.length ? '<span class="gl-base"><zh>基于</zh><en>based on</en> ' + s.base.map(name).join(' ') + '</span>' : '') + lineLink(pkg, s.line) + '</div><div class="gl-props">' + (props || '<span class="gl-none"><zh>没有属性</zh><en>no properties</en></span>') + '</div></li>';
      }).join('') + '</ul>');
    }
    if (otherRules.length) {
      h += block('other-rules', '<zh>其他规则</zh><en>Other rules</en>', otherRules.length, '<ul class="gl-list gl-rules">' + otherRules.map(function (s) {
        return '<li><pre class="gl-pre">' + C.hlSnippet(s.text) + '</pre>' + lineLink(pkg, s.line, s.endLine) + '</li>';
      }).join('') + '</ul>');
    }
    if (sets.length) {
      h += block('sets', '<zh>集合和补全</zh><en>Sets and fill</en>', sets.length,
        '<p class="gl-hint"><zh>集合由文件用 @ADD-TO、@REMOVE-FROM 增删；定义 @ATTACH 到集合上，就给集合的每个成员用上它。</zh><en>A file edits the sets with @ADD-TO and @REMOVE-FROM; a definition attached to a set with @ATTACH is applied to each of its members.</en></p>' +
        '<ul class="gl-list gl-rules">' + sets.map(function (s) {
          return '<li>' + (docText(s) ? '<p class="gl-doc">' + esc(docText(s)) + '</p>' : '') + '<pre class="gl-pre">' + C.hlSnippet(s.text) + '</pre>' + lineLink(pkg, s.line, s.endLine) + '</li>';
        }).join('') + '</ul>');
    }
    if (parts.length) {
      h += block('parts', '<zh>内部部件</zh><en>Private parts</en>', parts.length,
        '<p class="gl-hint"><zh>% 开头的符号只在包里用，画进别的符号里，文件不能直接用。</zh><en>Symbols named with % are private to the package: other symbols draw them, files cannot use them.</en></p>' +
        '<ul class="gl-list">' + parts.map(function (s) {
          var users = st.filter(function (u) { return u.kind === 'symbol' && (u.parts || []).indexOf(s.name) >= 0; }).map(function (u) { return u.name; });
          return '<li><div class="gl-li-head">' + name(s.name) + (users.length ? '<span class="gl-base"><zh>用在</zh><en>drawn in</en> ' + users.map(name).join(' ') + '</span>' : '') + lineLink(pkg, s.line, s.endLine) + '</div></li>';
        }).join('') + '</ul>');
    }
    if (aliases.length) {
      h += block('aliases', '<zh>别名</zh><en>Aliases</en>', aliases.length,
        '<div class="gl-aliases">' + aliases.map(function (a) { return '<span lang="zh-CN"><code>' + esc(a[0]) + '</code><b>→</b><code>' + esc(a[1]) + '</code></span>'; }).join('') + '</div>');
    }
    var lines = pkg.text.replace(/\n$/, '').split('\n');
    h += '<details class="gl-src" data-pkg="' + esc(pkg.name) + '"><summary><svg class="ico"><use href="#i-code"/></svg>' +
      fill('<zh>定义原文 · {f} · {n} 行</zh><en>The text · {f} · {n} lines</en>', { f: esc(pkg.file), n: lines.length }) + '</summary><div class="code gl-code-file"></div></details>';
    h += '</section>';
    return { html: h, count: found || 1 };
  }

  /* ---------------- the page ---------------- */

  /* Keep the common scene scale unless the complete drawing exceeds this card's inner area. */
  function fitCard(figure) {
    var svg = $('svg', figure);
    if (!svg) return;
    var style = getComputedStyle(figure);
    var width = figure.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    var height = figure.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
    var naturalWidth = parseFloat(svg.getAttribute('width')), naturalHeight = parseFloat(svg.getAttribute('height'));
    var ratio = Math.min(1, width / naturalWidth, height / naturalHeight);
    if (!(ratio > 0)) return;
    svg.style.width = (naturalWidth * ratio) + 'px';
    svg.style.height = (naturalHeight * ratio) + 'px';
    var badge = $('.gl-fit', figure.parentElement);
    badge.hidden = ratio >= 1;
    badge.textContent = fill('<zh>缩小显示 ×{r}</zh><en>Shown at ×{r}</en>', { r: ratio.toFixed(2) });
  }

  var cardSizes = global.ResizeObserver ? new ResizeObserver(function (entries) {
    entries.forEach(function (entry) { fitCard(entry.target); });
  }) : null;

  function fitCards() {
    if (cardSizes) cardSizes.disconnect();
    $$('.gl-fig', main).forEach(function (figure) {
      fitCard(figure);
      if (cardSizes) cardSizes.observe(figure);
    });
  }
  /* Invert the displayed SVG scale to keep the cross and dot at 10px and 3px. Reserve
     six screen pixels at the origin; a fixed scene margin can disappear on tiny previews. */
  function fitOrigin(svg) {
    var origin = $('.g-origin', svg);
    if (!origin) return;
    var base = svg._originViewBox || (svg._originViewBox = svg.getAttribute('viewBox').split(/\s+/).map(Number));
    svg.setAttribute('viewBox', base.join(' '));
    if (!state.show.origin) return;
    var rect = svg.getBoundingClientRect();
    if (rect.width <= 12 || rect.height <= 12) return;
    var point = origin.transform.baseVal.getItem(0).matrix;
    var padding = 6 / Math.min((rect.width - 12) / base[2], (rect.height - 12) / base[3]);
    var left = Math.min(base[0], point.e - padding), top = Math.min(base[1], point.f - padding);
    var right = Math.max(base[0] + base[2], point.e + padding), bottom = Math.max(base[1] + base[3], point.f + padding);
    svg.setAttribute('viewBox', [left, top, right - left, bottom - top].join(' '));
    var matrix = svg.getScreenCTM();
    if (matrix) $('.g-origin-mark', origin).setAttribute('transform', 'scale(' + (1 / Math.hypot(matrix.a, matrix.b)) + ')');
  }

  var originSizes = global.ResizeObserver ? new ResizeObserver(function (entries) {
    entries.forEach(function (entry) { fitOrigin(entry.target); });
  }) : null;

  function fitOrigins() {
    if (originSizes) originSizes.disconnect();
    $$('.gl-fig > svg, .gl-thumb > svg, #gl-dlg[open] .gl-dlg-fig > svg').forEach(function (svg) {
      fitOrigin(svg);
      if (originSizes) originSizes.observe(svg);
    });
  }
  if (!cardSizes || !originSizes) global.addEventListener('resize', function () {
    if (main) { fitCards(); fitOrigins(); }
  });

  function render() {
    var group = state.groups[state.group];
    if (!group) return;
    var symbols = symbolIndex(group);
    if (state.pkg && !group.packages.some(function (p) { return p.name === state.pkg; })) state.pkg = null;
    var shown = group.packages.filter(function (p) { return !state.pkg || p.name === state.pkg; });
    var html = '', count = 0;
    shown.forEach(function (pkg) {
      var r = renderPackage(group, pkg, symbols);
      html += r.html;
      count += r.count;
    });
    main.innerHTML = (terms().length && !count ? '<p class="gl-status"><zh>没有找到。</zh><en>Nothing found.</en></p>' : '') + html;
    main.className = 'gl-main' + (state.show.pins ? ' show-pins' : '') + (state.show.box ? ' show-box' : '') + (state.show.labels ? ' show-labels' : '') + (state.show.origin ? ' show-origin' : '');
    fitCards();
    fitOrigins();
    renderNav();
    $('#tb-series').textContent = groupTitle(group);
    $('#tb-packages').textContent = group.counts.packages;
    $('#tb-symbols').textContent = group.counts.symbols;
    $('#gl-group-title').textContent = groupTitle(group);
    $('#gl-group-counts').textContent = fill('<zh>{p} 个包 · {s} 个符号</zh><en>{p} packages · {s} symbols</en>', { p: group.counts.packages, s: group.counts.symbols });
    status.hidden = true;
  }

  function renderNav() {
    if (!state.index) return;
    var categories = [
      { kind: 'standard', title: '<zh>标准系列</zh><en>Standard series</en>' },
      { kind: 'extension', title: '<zh>扩展包</zh><en>Extensions</en>' },
      { kind: 'pdk', title: '<zh>工艺库</zh><en>PDKs</en>' }
    ];
    var html = '';
    categories.forEach(function (category) {
      var rows = '';
      state.index.groups.filter(function (g) { return g.kind === category.kind; }).forEach(function (g) {
        var title = groupTitle(g);
        var groupMatch = matches(title + ' ' + g.dir);
        var packages = g.packages.filter(function (p) { return groupMatch || matches(p.name + ' ' + p.version); });
        if (!groupMatch && !packages.length) return;
        var open = !!state.expanded[g.id] || (terms().length > 0 && !groupMatch);
        var selected = state.group === g.id;
        rows += '<li><div class="gl-group-row"><button type="button" class="gl-fold" data-fold="' + esc(g.id) +
          '" aria-label="' + esc(title) + '" aria-expanded="' + open + '" aria-controls="gl-children-' + esc(g.id) + '"><svg class="ico" aria-hidden="true"><use href="#i-next"/></svg></button>' +
          '<button type="button" class="gl-nav-item gl-group' + (selected ? ' here' : '') + '" data-group="' + esc(g.id) +
          '"' + (selected ? ' aria-current="true"' : '') + ' title="' + esc(title) + '"><span class="gl-nav-name">' + esc(title) + '</span>' +
          (g.kind === 'standard' && g.series === '0.2' ? '<small class="gl-readonly"><zh>只读</zh><en>read-only</en></small>' : '') +
          '<small class="gl-nav-count">' + g.counts.packages + '</small></button></div>' +
          '<ul class="gl-children" id="gl-children-' + esc(g.id) + '"' + (open ? '' : ' hidden') + '>' +
          '<li><button type="button" class="gl-nav-item' + (selected && !state.pkg ? ' here' : '') + '" data-group="' + esc(g.id) +
          '" data-pkg=""' + (selected && !state.pkg ? ' aria-current="true"' : '') + '><zh>全部</zh><en>All</en></button></li>' +
          packages.map(function (p) {
            var on = selected && state.pkg === p.name;
            return '<li><button type="button" class="gl-nav-item' + (on ? ' here' : '') + '" data-group="' + esc(g.id) +
              '" data-pkg="' + esc(p.name) + '"' + (on ? ' aria-current="true"' : '') + ' title="' + esc(p.name + ' ' + p.version) +
              '"><span class="gl-nav-name">' + esc(p.name) + '</span></button></li>';
          }).join('') + '</ul></li>';
      });
      if (rows) html += '<p class="toc-title">' + category.title + '</p><ul>' + rows + '</ul>';
    });
    $('#gl-nav').innerHTML = html || '<p class="gl-none"><zh>没有找到。</zh><en>Nothing found.</en></p>';
  }

  /* Show a group, and in it the package `pkg` alone when given */
  function choose(id, pkg) {
    state.group = id;
    state.pkg = pkg || null;
    state.expanded[id] = true;
    renderNav();
    if (state.groups[id]) { render(); return; }
    status.hidden = false;
    main.innerHTML = '';
    main.appendChild(status);
    load(id + '.js', function () { if (state.group === id) render(); }, function () { if (state.group === id) failed(); });
  }

  function failed() {
    status.hidden = false;
    status.innerHTML = '<zh>图库的数据没有生成。在仓库根目录先构建 WebAssembly 核心，再运行 <code>node site/gallery/build.mjs</code>（见 site/README.md）。</zh><en>The gallery’s data has not been built. From the repository root, build the WebAssembly core, then run <code>node site/gallery/build.mjs</code> (see site/README.md).</en>';
  }

  /* The source of a package, highlighted when it is first opened */
  function paintSource(det) {
    var box = $('.gl-code-file', det);
    if (box.childNodes.length) return;
    var pkg = state.groups[state.group].packages.filter(function (p) { return p.name === det.getAttribute('data-pkg'); })[0];
    var lines = pkg.text.replace(/\n$/, '').split('\n');
    var hl = C.hlPackage(lines);
    box.innerHTML = lines.map(function (l, i) {
      return '<div class="cl" id="' + esc(pkg.name) + '-L' + (i + 1) + '"><span class="ln" aria-hidden="true">' + (i + 1) + '</span><span class="tx">' + (hl[i] || ' ') + '</span></div>';
    }).join('');
  }

  /* ---------------- a symbol, in detail ---------------- */

  function openSymbol(pkgName, symName) {
    var group = state.groups[state.group];
    var pkg = group.packages.filter(function (p) { return p.name === pkgName; })[0];
    var s = pkg && pkg.statements.filter(function (x) { return x.name === symName && x.kind === 'symbol'; })[0];
    if (!s) return;
    var rules = [];
    group.packages.forEach(function (p) {
      p.statements.forEach(function (r) { if (r.kind === 'rule' && r.action === 'symbol' && r.target === symName) rules.push({ pkg: p, r: r }); });
    });
    var big = symbolSvg(s).replace(/ width="([\d.]+)" height="([\d.]+)"/, function (m, w, hh) { return ' width="' + (w * 2) + '" height="' + (hh * 2) + '"'; });
    var h = '<div class="gl-dlg-grid"><div class="gl-dlg-fig ' + main.className.replace('gl-main', '') + '">' + big + '</div><div class="gl-dlg-info">';
    h += '<p class="gl-dlg-pkg"><zh>包</zh><en>Package</en> ' + name(pkg.name + ' ' + pkg.version) + ' · ' + lineLink(pkg, s.line, s.endLine) + '</p>';
    if (docText(s)) h += '<p class="gl-doc">' + esc(docText(s)) + '</p>';
    h += '<table class="gl-table gl-pins-table"><thead><tr><th><zh>端子</zh><en>Terminal</en></th><th><zh>名字</zh><en>Name</en></th><th><zh>方向</zh><en>Direction</en></th><th><zh>位置</zh><en>At</en></th></tr></thead><tbody>' +
      (s.pins.length ? s.pins.map(function (p) {
        return '<tr><td class="mono">' + esc(terminalText(p)) + '</td><td class="mono">' + esc(p.label || p.name || '') + '</td><td>' + (ARROWS[p.dir] || '') + ' ' + (DIRS[p.dir] || '—') + '</td><td class="mono">(' + p.at.join(', ') + ')</td></tr>';
      }).join('') : '<tr><td colspan="4" class="gl-none"><zh>没有引脚</zh><en>no pins</en></td></tr>') + '</tbody></table>';
    h += '<dl class="gl-meta">';
    if (s.box) h += '<div><dt><zh>选择框</zh><en>Box</en></dt><dd class="mono">box=[' + s.box.join(', ') + ']</dd></div>';
    if (s.labels && s.labels.length) {
      h += '<div><dt><zh>标签</zh><en>Labels</en></dt><dd>' + s.labels.map(function (l) { return '<span class="gl-prop">' + esc(l.key) + ' = <b>' + esc(l.text) + '</b></span>'; }).join(' ') + '</dd></div>';
    }
    if (s.parts && s.parts.length) h += '<div><dt><zh>由这些部件组成</zh><en>Drawn from</en></dt><dd>' + s.parts.map(name).join(' ') + '</dd></div>';
    if (rules.length) {
      h += '<div><dt><zh>谁会配上它</zh><en>Given by</en></dt><dd>' + rules.map(function (x) { return '<div class="gl-given">' + code(x.r.select) + ' <span class="gl-from">' + esc(x.pkg.name) + '</span></div>'; }).join('') + '</dd></div>';
    }
    h += '</dl>';
    if (s.problems && s.problems.length) h += '<p class="gl-problem"><svg class="ico"><use href="#i-warn"/></svg>' + s.problems.map(esc).join('<br>') + '</p>';
    h += '</div></div>';
    h += '<h4 class="gl-dlg-src"><zh>定义原文</zh><en>The definition</en></h4><div class="code gl-code-file">' +
      s.text.split('\n').map(function (l, i) {
        return '<div class="cl"><span class="ln" aria-hidden="true">' + (s.line + i) + '</span><span class="tx">' + (C.hlSnippet(l) || ' ') + '</span></div>';
      }).join('') + '</div>';
    $('#gl-dlg-title').innerHTML = '<code>' + esc(s.name) + '</code>';
    $('#gl-dlg-body').innerHTML = h;
    var dlg = $('#gl-dlg');
    if (dlg.showModal) dlg.showModal(); else dlg.setAttribute('open', '');
    fitOrigins();
  }

  /* ---------------- state in the address ---------------- */

  function saveUrl(push) {
    var params = [];
    if (state.group && state.index && state.group !== state.index.groups[0].id) params.push('series=' + encodeURIComponent(state.group));
    if (state.q) params.push('q=' + encodeURIComponent(state.q));
    if (state.pkg) params.push('package=' + encodeURIComponent(state.pkg));
    var url = location.pathname + (params.length ? '?' + params.join('&') : '') + location.hash;
    try { if (url !== location.pathname + location.search + location.hash) history[push ? 'pushState' : 'replaceState'](null, '', url); } catch (e) { /* opened from the disk */ }
    var lang = $('[data-lang-link]');
    if (lang) lang.setAttribute('href', lang.getAttribute('href').split(/[?#]/)[0] + (params.length ? '?' + params.join('&') : ''));
  }

  function readUrl() {
    var params = new URLSearchParams(location.search);
    return { series: params.get('series'), package: params.get('package'), q: params.get('q') || '' };
  }

  function restoreUrl() {
    var url = readUrl();
    state.q = url.q;
    $('#gl-q').value = state.q;
    var first = state.index.groups.filter(function (g) { return g.id === url.series; })[0] || state.index.groups[0];
    if (first) choose(first.id, url.package);
    saveUrl();
  }

  /* ---------------- start ---------------- */

  function boot() {
    main = $('#gl-main');
    status = $('#gl-status');
    var stored = P.store('rnl-gallery-show');
    if (stored) { try { var s = JSON.parse(stored); ['pins', 'box', 'labels', 'origin'].forEach(function (k) { if (typeof s[k] === 'boolean') state.show[k] = s[k]; }); } catch (e) { /* ignore */ } }
    $$('.gl-tog').forEach(function (b) {
      var k = b.getAttribute('data-show');
      b.setAttribute('aria-pressed', state.show[k] ? 'true' : 'false');
      b.addEventListener('click', function () {
        state.show[k] = !state.show[k];
        b.setAttribute('aria-pressed', state.show[k] ? 'true' : 'false');
        P.store('rnl-gallery-show', JSON.stringify(state.show));
        main.classList.toggle('show-' + k, state.show[k]);
        var detail = $('.gl-dlg-fig');
        if (detail) detail.classList.toggle('show-' + k, state.show[k]);
        if (k === 'origin') fitOrigins();
      });
    });
    var input = $('#gl-q');
    state.q = readUrl().q;
    input.value = state.q;
    input.addEventListener('input', P.debounce(function () {
      state.q = input.value.trim();
      renderNav();
      render();
      saveUrl();
    }, 120));
    $('#gl-nav').addEventListener('click', function (e) {
      var fold = e.target.closest('button[data-fold]');
      if (fold) {
        var id = fold.getAttribute('data-fold');
        var open = fold.getAttribute('aria-expanded') !== 'true';
        state.expanded[id] = open;
        fold.setAttribute('aria-expanded', String(open));
        document.getElementById(fold.getAttribute('aria-controls')).hidden = !open;
        return;
      }
      var b = e.target.closest('button[data-group]');
      if (!b) return;
      P.closeToc();
      choose(b.getAttribute('data-group'), b.getAttribute('data-pkg'));
      saveUrl(true);
      var top = $('#gallery').getBoundingClientRect().top + window.scrollY - 60;
      if (window.scrollY > top) window.scrollTo(0, top);
    });
    window.addEventListener('popstate', function () { if (state.index) restoreUrl(); });
    main.addEventListener('click', function (e) {
      var card = e.target.closest('[data-sym]');
      if (card) { openSymbol(card.getAttribute('data-pkg'), card.getAttribute('data-sym')); return; }
      var go = e.target.closest('[data-goto]');
      if (go && state.pkg) { state.pkg = null; render(); saveUrl(true); }
    });
    main.addEventListener('toggle', function (e) { if (e.target.classList && e.target.classList.contains('gl-src') && e.target.open) paintSource(e.target); }, true);
    var dlg = $('#gl-dlg');
    $('#gl-dlg-close').addEventListener('click', function () { dlg.close ? dlg.close() : dlg.removeAttribute('open'); });
    dlg.addEventListener('click', function (e) {
      if (e.target === dlg) dlg.close();
      var t = e.target.closest('[data-sym]');
      if (t) openSymbol(t.getAttribute('data-pkg'), t.getAttribute('data-sym'));
    });
    load('index.js', restoreUrl, failed);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})(window);
