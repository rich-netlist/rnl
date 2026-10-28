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
  /* An entry of one file opens in the web viewer with ?url=; an entry of several files with ?manifest=, the address
     of its manifest, from which the viewer reads all of its files. */
  var TIMEOUT = 8000;
  var KEY = 'rnl-gallery-index';
  var locale = document.documentElement.getAttribute('data-locale');

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

  /* The authors of an entry: a name, or a list of {name, url}; a link is kept only when it is an https address */
  function authorsOf(entry) {
    var a = entry.author;
    if (typeof a === 'string') return a ? [{ name: a }] : [];
    if (!Array.isArray(a)) return [];
    return a.filter(function (x) { return x && typeof x.name === 'string' && x.name; }).map(function (x) {
      return { name: x.name, url: typeof x.url === 'string' && /^https:\/\/[^\s"<>]+$/.test(x.url) ? x.url : null };
    });
  }

  /* A tag is written as it is to be shown ("180nm CMOS", "PLL"): it is shown so in every language, and compared by its key,
     which ignores case and turns spaces into hyphens (180nm-cmos), so that PLL and pll are one tag */
  var TAG = /^[A-Za-z0-9](?:[A-Za-z0-9.+\/-]| (?! ))*$/;
  function tagKey(tag) { return tag.toLowerCase().replace(/ /g, '-'); }
  /* The written forms of the tags of all entries, by key: the one most entries use is the label (the first seen when equal) */
  var written = {};
  function tagLabel(key) { return written[key] ? written[key].label : key; }
  function collectLabels() {
    var seen = {};
    state.entries.forEach(function (e) {
      (Array.isArray(e.tags) ? e.tags : []).forEach(function (t) {
        if (typeof t !== 'string' || t.length > 32 || !TAG.test(t) || / $/.test(t)) return;
        var k = tagKey(t);
        var forms = seen[k] || (seen[k] = { order: [], count: {} });
        if (!forms.count[t]) { forms.count[t] = 0; forms.order.push(t); }
        forms.count[t]++;
      });
    });
    written = {};
    Object.keys(seen).forEach(function (k) {
      var best = seen[k].order.reduce(function (a, b) { return seen[k].count[b] > seen[k].count[a] ? b : a; });
      written[k] = { label: best };
    });
  }
  /* The keys of the tags of an entry, each once, in the order written */
  function tagsOf(entry) {
    var keys = [];
    (Array.isArray(entry.tags) ? entry.tags : []).forEach(function (t) {
      if (typeof t !== 'string' || t.length > 32 || !TAG.test(t) || / $/.test(t)) return;
      var k = tagKey(t);
      if (keys.indexOf(k) < 0) keys.push(k);
    });
    return keys;
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
    if (v.manifest && manifestUrl(v.manifest)) return VIEWER + '?manifest=' + encodeURIComponent(manifestUrl(v.manifest));
    return null;
  }

  function haystack(entry) {
    return [pick(entry.title), (entry.title || {}).en, pick(entry.description), (entry.description || {}).en, authorsOf(entry).map(function (a) { return a.name; }).join(' '), entry.license, entry.source && entry.source.repo, tagsOf(entry).map(tagLabel).join(' ')].join(' ').toLowerCase();
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

  /* A tag on a card: a button that chooses it */
  function tagButton(tag) {
    var b = el('button', 'cg-tag', tagLabel(tag));
    b.type = 'button';
    b.setAttribute('data-tag', tag);
    b.setAttribute('aria-pressed', state.tags.indexOf(tag) >= 0 ? 'true' : 'false');
    b.addEventListener('click', function () { toggleTag(tag); });
    return b;
  }

  function card(entry) {
    var title = pick(entry.title) || entry.id;
    var href = openUrl(entry);
    var article = el('article', 'cg-card');
    article.setAttribute('data-id', entry.id);
    var fig = el(href ? 'a' : 'div', 'cg-fig');
    if (href) { fig.href = href; fig.target = '_blank'; fig.rel = 'noopener'; fig.setAttribute('aria-label', title); }
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
    if (href) { var a = el('a', '', title); a.href = href; a.target = '_blank'; a.rel = 'noopener'; h.appendChild(a); } else h.textContent = title;
    body.appendChild(h);
    body.appendChild(el('p', 'cg-desc', pick(entry.description)));
    var by = el('p', 'cg-by');
    authorsOf(entry).forEach(function (author, i) {
      if (i) by.appendChild(document.createTextNode(', '));
      if (author.url) {
        var who = el('a', '', author.name);
        who.href = author.url;
        who.target = '_blank';
        who.rel = 'noopener noreferrer';
        by.appendChild(who);
      } else by.appendChild(document.createTextNode(author.name));
    });
    body.appendChild(by);

    var meta = el('p', 'cg-meta');
    if (entry.license) meta.appendChild(el('span', 'chip', entry.license));
    if (entry.files > 1) meta.appendChild(el('span', 'cg-files', fill('<en>{n} files</en>', { n: entry.files })));
    var s = entry.source && sourceUrl(entry.source.url);
    if (s) {
      var link = el('a', 'cg-src', entry.source.repo + '@' + String(entry.source.commit || '').slice(0, 7));
      link.href = s;
      link.rel = 'noopener noreferrer';
      link.title = fill('<en>See the source on GitHub</en>');
      meta.appendChild(link);
    }
    body.appendChild(meta);
    var tags = tagsOf(entry);
    if (tags.length) {
      var row = el('p', 'cg-tagrow');
      tags.forEach(function (t) { row.appendChild(tagButton(t)); });
      body.appendChild(row);
    }
    article.appendChild(body);
    return article;
  }

  /* The tags of all entries under the search box, most used first, each a check box with the number of entries that
     have it; the ones asked for stay in the list even when no entry has them. Together they are an intersection. */
  function renderTagBar() {
    collectLabels();
    var count = {};
    state.entries.forEach(function (e) { tagsOf(e).forEach(function (t) { count[t] = (count[t] || 0) + 1; }); });
    state.tags.forEach(function (t) { if (!count[t]) count[t] = 0; });
    var names = Object.keys(count).sort(function (a, b) {
      var la = tagLabel(a).toLowerCase(), lb = tagLabel(b).toLowerCase();
      return count[b] - count[a] || (la < lb ? -1 : la > lb ? 1 : 0);
    });
    tagBar.textContent = '';
    tagBar.hidden = !names.length;
    names.forEach(function (t) {
      var label = el('label', 'cg-check');
      var box = el('input');
      box.type = 'checkbox';
      box.checked = state.tags.indexOf(t) >= 0;
      box.setAttribute('data-tag', t);
      box.addEventListener('change', function () { toggleTag(t); });
      label.appendChild(box);
      label.appendChild(el('span', 'cg-check-name', tagLabel(t)));
      label.appendChild(el('small', '', String(count[t])));
      tagBar.appendChild(label);
    });
    if (state.tags.length) {
      var clear = el('button', 'cg-clear', fill('<en>Clear tags</en>'));
      clear.type = 'button';
      clear.addEventListener('click', function () { state.tags = []; render(); saveUrl(); });
      tagBar.appendChild(clear);
    }
  }

  function render() {
    collectLabels();
    var shown = state.entries.filter(matches);
    main.textContent = '';
    if (!state.entries.length) {
      main.appendChild(el('p', 'gl-status', fill('<en>There are no entries yet.</en>')));
    } else if (!shown.length) {
      main.appendChild(el('p', 'gl-status', fill('<en>Nothing found.</en>')));
    } else {
      var grid = el('div', 'cg-grid');
      shown.forEach(function (e) { grid.appendChild(card(e)); });
      main.appendChild(grid);
    }
    renderTagBar();
    counts.textContent = state.entries.length ? fill('<en>{n} entries</en>', { n: shown.length }) +
      (shown.length !== state.entries.length ? fill('<en> of {t}</en>', { t: state.entries.length }) : '') : '';
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
        notice(fill('<en>The gallery’s data is newer than this page. Reload the page.</en>'), 'error');
      } else if (old) {
        state.entries = old;
        render();
        notice(fill('<en>The latest gallery cannot be loaded right now; showing the cached copy from your last visit.</en>'), 'warn');
      } else {
        main.textContent = '';
        var p = el('p', 'gl-status cg-notice cg-error');
        p.appendChild(document.createTextNode(fill('<en>The gallery cannot be read right now. The list is on GitHub: </en>')));
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
    state.tags = params.getAll('tag').map(function (t) { return tagKey(t.trim()); }).filter(function (t, i, all) { return /^[a-z0-9][a-z0-9.+\/-]{0,31}$/.test(t) && all.indexOf(t) === i; });
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
