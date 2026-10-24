/* RNL gallery · the circuit gallery (gallery.html)
 *
 * Reads index.json and the thumbnails from the GitHub Pages of rich-netlist/rnl-gallery, so that a new entry
 * shows up a few minutes after it is merged there, with no release of this site. Nothing else is read: no GitHub
 * API, no netlists. An entry opens in the web viewer, which reads the design's files from its own repository.
 * The index is the work of reviewed pull requests, but what it holds is still only ever shown as text.
 */
(function (global) {
  'use strict';

  var P = global.RNLPage;
  var $ = P.$, fill = P.fill;

  var DATA = 'https://rich-netlist.github.io/rnl-gallery/';
  var VIEWER = '../viewer/';
  /* Whether the web viewer opens an entry of several files from its manifest (?manifest=). Until it does, such an
     entry shows its thumbnail and its source only. An entry of one file opens with ?url=. */
  var MANIFEST = false;
  var TIMEOUT = 8000;
  var KEY = 'rnl-gallery-index';
  var locale = document.documentElement.getAttribute('data-locale');

  /* The suggested tags of rnl-gallery, in this page's language; any other tag shows as it is written */
  var TAGS = {
    amplifier: '<zh>放大器</zh><en>amplifier</en>', opamp: '<zh>运放</zh><en>opamp</en>', ota: 'OTA',
    comparator: '<zh>比较器</zh><en>comparator</en>', bandgap: '<zh>带隙基准</zh><en>bandgap</en>',
    reference: '<zh>基准</zh><en>reference</en>', bias: '<zh>偏置</zh><en>bias</en>', ldo: 'LDO',
    power: '<zh>电源</zh><en>power</en>', oscillator: '<zh>振荡器</zh><en>oscillator</en>',
    vco: '<zh>压控振荡器</zh><en>VCO</en>', pll: '<zh>锁相环</zh><en>PLL</en>', adc: 'ADC', 'sar-adc': 'SAR ADC', dac: 'DAC',
    filter: '<zh>滤波器</zh><en>filter</en>', 'switched-capacitor': '<zh>开关电容</zh><en>switched-capacitor</en>',
    'mixed-signal': '<zh>混合信号</zh><en>mixed-signal</en>', digital: '<zh>数字</zh><en>digital</en>',
    rf: '<zh>射频</zh><en>RF</en>', lna: 'LNA', mixer: '<zh>混频器</zh><en>mixer</en>',
    hierarchical: '<zh>层次化</zh><en>hierarchical</en>', 'transistor-level': '<zh>晶体管级</zh><en>transistor-level</en>',
    'virtuoso-export': '<zh>Virtuoso 导出</zh><en>Virtuoso export</en>'
  };

  var state = { entries: [], q: '', tags: [] };
  var main, counts, tagBar;

  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  }

  /* A text of the entry in the page's language, else English */
  function pick(texts) {
    if (!texts || typeof texts !== 'object') return '';
    return String(Object.prototype.hasOwnProperty.call(texts, locale) ? texts[locale] : texts.en || '');
  }

  function tagLabel(tag) { return Object.prototype.hasOwnProperty.call(TAGS, tag) ? TAGS[tag] : tag; }
  function tagsOf(entry) {
    return Array.isArray(entry.tags) ? entry.tags.filter(function (t) { return typeof t === 'string' && /^[a-z0-9][a-z0-9-]{0,31}$/.test(t); }) : [];
  }

  /* What an entry may point to: the paths and addresses the index is allowed to give */
  function thumbUrl(path) { return /^thumbs\/[A-Za-z0-9._-]+\.png$/.test(path) ? DATA + path : null; }
  function manifestUrl(path) { return /^manifests\/[A-Za-z0-9._-]+\.json$/.test(path) ? DATA + path : null; }
  function sourceUrl(url) { return typeof url === 'string' && /^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+(\/|$)/.test(url) ? url : null; }
  function rawUrl(url) { return typeof url === 'string' && /^https:\/\/raw\.githubusercontent\.com\/[^\s]+$/.test(url) ? url : null; }

  /* The address of the viewer opened on an entry, or null when it cannot yet */
  function openUrl(entry) {
    var v = entry.viewer || {};
    var level = typeof v.level === 'string' && v.level ? '&level=' + encodeURIComponent(v.level) : '';
    if (rawUrl(v.url)) return VIEWER + '?url=' + encodeURIComponent(v.url) + level;
    if (MANIFEST && v.manifest && manifestUrl(v.manifest)) return VIEWER + '?manifest=' + encodeURIComponent(manifestUrl(v.manifest));
    return null;
  }

  function haystack(entry) {
    return [pick(entry.title), (entry.title || {}).en, pick(entry.description), (entry.description || {}).en, entry.author, entry.license, entry.source && entry.source.repo, tagsOf(entry).join(' ')].join(' ').toLowerCase();
  }

  function matches(entry) {
    var have = tagsOf(entry);
    if (!state.tags.every(function (t) { return have.indexOf(t) >= 0; })) return false;
    var words = state.q.toLowerCase().split(/\s+/).filter(Boolean);
    if (!words.length) return true;
    var text = haystack(entry);
    return words.every(function (w) { return text.indexOf(w) >= 0; });
  }

  function toggleTag(tag) {
    var at = state.tags.indexOf(tag);
    if (at >= 0) state.tags.splice(at, 1); else state.tags.push(tag);
    render();
    saveUrl();
  }

  function tagButton(tag, count) {
    var b = el('button', 'cg-tag', tagLabel(tag));
    b.type = 'button';
    b.setAttribute('data-tag', tag);
    b.setAttribute('aria-pressed', state.tags.indexOf(tag) >= 0 ? 'true' : 'false');
    if (count !== undefined) b.appendChild(el('small', '', String(count)));
    b.addEventListener('click', function () { toggleTag(tag); });
    return b;
  }

  function card(entry) {
    var title = pick(entry.title) || entry.id;
    var href = openUrl(entry);
    var article = el('article', 'cg-card');
    article.setAttribute('data-id', entry.id);
    var fig = el(href ? 'a' : 'div', 'cg-fig');
    if (href) { fig.href = href; fig.setAttribute('aria-label', title); }
    var src = entry.thumb && thumbUrl(entry.thumb.path);
    if (src) {
      var img = el('img');
      img.src = src;
      img.alt = '';
      img.loading = 'lazy';
      img.decoding = 'async';
      if (entry.thumb.width > 0 && entry.thumb.height > 0) { img.width = entry.thumb.width; img.height = entry.thumb.height; }
      img.addEventListener('error', function () { fig.classList.add('cg-nothumb'); img.remove(); fig.appendChild(el('span', 'cg-nothumb-text', title)); });
      fig.appendChild(img);
    } else {
      fig.classList.add('cg-nothumb');
      fig.appendChild(el('span', 'cg-nothumb-text', title));
    }
    article.appendChild(fig);

    var body = el('div', 'cg-body');
    var h = el('h3', 'cg-title');
    if (href) { var a = el('a', '', title); a.href = href; h.appendChild(a); } else h.textContent = title;
    body.appendChild(h);
    body.appendChild(el('p', 'cg-desc', pick(entry.description)));
    body.appendChild(el('p', 'cg-by', entry.author || ''));

    var meta = el('p', 'cg-meta');
    if (entry.license) meta.appendChild(el('span', 'chip', entry.license));
    if (entry.files > 1) meta.appendChild(el('span', 'cg-files', fill('<zh>{n} 个文件</zh><en>{n} files</en>', { n: entry.files })));
    var s = entry.source && sourceUrl(entry.source.url);
    if (s) {
      var link = el('a', 'cg-src', entry.source.repo + '@' + String(entry.source.commit || '').slice(0, 7));
      link.href = s;
      link.rel = 'noopener noreferrer';
      link.title = fill('<zh>在 GitHub 上看来源</zh><en>See the source on GitHub</en>');
      meta.appendChild(link);
    }
    body.appendChild(meta);
    var tags = tagsOf(entry);
    if (tags.length) {
      var row = el('p', 'cg-tagrow');
      tags.forEach(function (t) { var b = tagButton(t); b.classList.add('cg-tag-sm'); row.appendChild(b); });
      body.appendChild(row);
    }
    if (!href) body.appendChild(el('p', 'cg-note', fill('<zh>这个设计有多个文件，查看器还不能一次打开它；先看缩略图和来源。</zh><en>This design has several files, which the viewer cannot open in one go yet; see the thumbnail and the source.</en>')));
    article.appendChild(body);
    return article;
  }

  /* The tags of all entries, most used first; the ones asked for stay in the bar even when no entry has them */
  function renderTagBar() {
    var count = {};
    state.entries.forEach(function (e) { tagsOf(e).forEach(function (t) { count[t] = (count[t] || 0) + 1; }); });
    state.tags.forEach(function (t) { if (!count[t]) count[t] = 0; });
    var names = Object.keys(count).sort(function (a, b) { return count[b] - count[a] || (a < b ? -1 : 1); });
    tagBar.textContent = '';
    tagBar.hidden = !names.length;
    names.forEach(function (t) { tagBar.appendChild(tagButton(t, count[t])); });
    if (state.tags.length) {
      var clear = el('button', 'cg-clear', fill('<zh>清除标签</zh><en>Clear tags</en>'));
      clear.type = 'button';
      clear.addEventListener('click', function () { state.tags = []; render(); saveUrl(); });
      tagBar.appendChild(clear);
    }
  }

  function render() {
    var shown = state.entries.filter(matches);
    main.textContent = '';
    if (!state.entries.length) {
      main.appendChild(el('p', 'gl-status', fill('<zh>图库里还没有条目。</zh><en>There are no entries yet.</en>')));
    } else if (!shown.length) {
      main.appendChild(el('p', 'gl-status', fill('<zh>没有找到。</zh><en>Nothing found.</en>')));
    } else {
      var grid = el('div', 'cg-grid');
      shown.forEach(function (e) { grid.appendChild(card(e)); });
      main.appendChild(grid);
    }
    renderTagBar();
    counts.textContent = state.entries.length ? fill('<zh>{n} 个条目</zh><en>{n} entries</en>', { n: shown.length }) +
      (shown.length !== state.entries.length ? fill('<zh>，共 {t} 个</zh><en> of {t}</en>', { t: state.entries.length }) : '') : '';
  }

  function notice(text, kind) {
    var p = el('p', 'gl-status cg-notice' + (kind ? ' cg-' + kind : ''), text);
    main.insertBefore(p, main.firstChild);
  }

  /* The index, from the network; its entries need not be usable, so each is checked where it is used */
  function fetchIndex() {
    var controller = global.AbortController ? new AbortController() : null;
    var timer = setTimeout(function () { if (controller) controller.abort(); }, TIMEOUT);
    return fetch(DATA + 'index.json', { cache: 'no-cache', signal: controller ? controller.signal : undefined }).then(function (r) {
      clearTimeout(timer);
      if (!r.ok) throw new Error('HTTP ' + r.status);
      return r.text();
    }, function (e) { clearTimeout(timer); throw e; });
  }

  function accept(text) {
    var data = JSON.parse(text);
    if (!data || typeof data !== 'object' || !Array.isArray(data.entries)) throw new Error('not an index');
    if (data.format !== 1) { var err = new Error('format'); err.format = data.format; throw err; }
    return data.entries.filter(function (e) { return e && typeof e === 'object' && typeof e.id === 'string'; });
  }

  function load() {
    fetchIndex().then(function (text) {
      state.entries = accept(text);
      P.store(KEY, text);
      render();
    }).catch(function (error) {
      var old = null;
      if (!error.format) { try { var saved = P.store(KEY); if (saved) old = accept(saved); } catch (e) { old = null; } }
      if (error.format) {
        main.textContent = '';
        notice(fill('<zh>图库的数据比这一页新，请刷新页面。</zh><en>The gallery’s data is newer than this page. Reload the page.</en>'), 'error');
      } else if (old) {
        state.entries = old;
        render();
        notice(fill('<zh>暂时读不到图库的最新内容，这是上次读到的。</zh><en>The latest gallery cannot be read right now; this is what was read last time.</en>'), 'warn');
      } else {
        main.textContent = '';
        var p = el('p', 'gl-status cg-notice cg-error');
        p.appendChild(document.createTextNode(fill('<zh>暂时读不到图库。可以直接去 GitHub 上看列表：</zh><en>The gallery cannot be read right now. The list is on GitHub: </en>')));
        var a = el('a', '', 'rich-netlist/rnl-gallery');
        a.href = 'https://github.com/rich-netlist/rnl-gallery/tree/main/entries';
        p.appendChild(a);
        main.appendChild(p);
        counts.textContent = '';
      }
    });
  }

  /* The search and the tags are in the address (?q=…&tag=a&tag=b), so that a view can be shared */
  function query() {
    var parts = [];
    if (state.q) parts.push('q=' + encodeURIComponent(state.q));
    state.tags.forEach(function (t) { parts.push('tag=' + encodeURIComponent(t)); });
    return parts.length ? '?' + parts.join('&') : '';
  }

  function saveUrl() {
    var url = location.pathname + query() + location.hash;
    try { if (url !== location.pathname + location.search + location.hash) history.replaceState(null, '', url); } catch (e) { /* opened from the disk */ }
    document.querySelectorAll('[data-lang-link]').forEach(function (lang) {
      lang.setAttribute('href', lang.getAttribute('href').split(/[?#]/)[0] + query());
    });
  }

  function boot() {
    main = $('#cg-main');
    counts = $('#cg-counts');
    tagBar = $('#cg-tags');
    var input = $('#cg-q');
    var params = new URLSearchParams(location.search);
    state.q = params.get('q') || '';
    state.tags = params.getAll('tag').filter(function (t, i, all) { return /^[a-z0-9][a-z0-9-]{0,31}$/.test(t) && all.indexOf(t) === i; });
    input.value = state.q;
    input.addEventListener('input', P.debounce(function () {
      state.q = input.value.trim();
      render();
      saveUrl();
    }, 120));
    saveUrl();
    load();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})(window);
