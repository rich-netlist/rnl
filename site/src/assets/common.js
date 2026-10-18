/* RNL introduction pages · shared helpers, theme, navigation, tabs and static code snippets */
(function (global) {
  'use strict';

  var C = global.RNLCode;

  function $(s, r) { return (r || document).querySelector(s); }
  function $$(s, r) { return Array.prototype.slice.call((r || document).querySelectorAll(s)); }
  function icon(name, cls) {
    return '<svg class="ico' + (cls ? ' ' + cls : '') + '" aria-hidden="true"><use href="#i-' + name + '"></use></svg>';
  }
  function store(k, v) {
    try {
      if (v === undefined) return localStorage.getItem(k);
      localStorage.setItem(k, v);
    } catch (e) { /* ignore when storage is unavailable */ }
    return null;
  }
  function debounce(fn, ms) {
    var t = null;
    return function () { clearTimeout(t); t = setTimeout(fn, ms); };
  }
  function onVisible(el, cb, threshold) {
    if (!el) return;
    if (!('IntersectionObserver' in window)) { cb(true); return; }
    var io = new IntersectionObserver(function (es) {
      es.forEach(function (e) { cb(e.isIntersecting); });
    }, { threshold: threshold || 0.2 });
    io.observe(el);
  }
  function setPressed(buttons, active) {
    buttons.forEach(function (b) { b.setAttribute('aria-pressed', b === active ? 'true' : 'false'); });
  }
  /* Fill {name} placeholders, so each language can order a phrase its own way */
  function fill(t, v) {
    return t.replace(/\{(\w+)\}/g, function (_, k) { return v[k]; });
  }

  /* Highlight static code snippets: the data-hl attribute */
  function paint(root) {
    $$('[data-hl]', root).forEach(function (el) {
      var keep = el.innerHTML;
      el.innerHTML = C.hlAuto(el.getAttribute('data-hl')) + keep;
      el.removeAttribute('data-hl');
    });
  }

  /* ================= Theme and navigation ================= */

  function initTheme() {
    var btn = $('#theme-btn');
    if (!btn) return;
    var mq = window.matchMedia ? window.matchMedia('(prefers-color-scheme: dark)') : null;
    function effective() {
      var t = document.documentElement.getAttribute('data-theme');
      if (t) return t;
      return mq && mq.matches ? 'dark' : 'light';
    }
    function paintBtn() {
      var dark = effective() === 'dark';
      btn.innerHTML = icon(dark ? 'sun' : 'moon');
      btn.setAttribute('aria-label', dark ? '<zh>切换到浅色</zh><en>Switch to light theme</en>' : '<zh>切换到深色</zh><en>Switch to dark theme</en>');
    }
    btn.addEventListener('click', function () {
      var next = effective() === 'dark' ? 'light' : 'dark';
      document.documentElement.setAttribute('data-theme', next);
      store('rnl-intro-theme', next);
      paintBtn();
    });
    if (mq && mq.addEventListener) mq.addEventListener('change', paintBtn);
    paintBtn();
  }

  function initNav() {
    var links = $$('.toc a[href^="#"]');
    var map = {};
    links.forEach(function (a) { map[a.getAttribute('href').slice(1)] = a; });
    if ('IntersectionObserver' in window) {
      var io = new IntersectionObserver(function (es) {
        es.forEach(function (e) {
          if (!e.isIntersecting) return;
          links.forEach(function (a) { a.classList.remove('is-on'); });
          var a = map[e.target.id];
          if (a) a.classList.add('is-on');
        });
      }, { rootMargin: '-40% 0px -55% 0px' });
      Object.keys(map).forEach(function (id) {
        var s = document.getElementById(id);
        if (s) io.observe(s);
      });
    }
    var bar = $('#progress-bar');
    function onScroll() {
      var h = document.documentElement;
      var p = h.scrollTop / Math.max(1, h.scrollHeight - h.clientHeight);
      if (bar) bar.style.width = (Math.max(0, Math.min(1, p)) * 100).toFixed(2) + '%';
    }
    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
    /* Opened from disk, a link to a folder would show a directory listing: point it at the page itself */
    if (location.protocol === 'file:') {
      $$('a[href]').forEach(function (a) {
        var h = a.getAttribute('href');
        if (!/^([a-z]+:|#)/i.test(h) && /(^|\/)$|^\.\.?$/.test(h.split('#')[0])) {
          var parts = h.split('#');
          a.setAttribute('href', parts[0].replace(/\/?$/, '/') + 'index.html' + (parts[1] !== undefined ? '#' + parts[1] : ''));
        }
      });
    }
    /* Switching language keeps the reader on the same section */
    $$('[data-lang-link]').forEach(function (a) {
      var base = a.getAttribute('href');
      a.addEventListener('click', function () {
        var on = $('.toc a.is-on');
        a.setAttribute('href', base + (on ? on.getAttribute('href') : ''));
      });
    });
    /* A top bar whose links do not fit beside the brand draws them closer together, then writes the links
       that have a short form (a span with the class s) in it, and when they still do not fit, folds them into
       the menu, as on narrow screens */
    var topbar = $('.topbar'), row = $('.topbar-in');
    function fit() {
      if (!topbar || !row) return;
      var over = function () { return row.scrollWidth > row.clientWidth + 1; };
      topbar.classList.remove('is-tight', 'is-short', 'is-compact');
      if (!over()) return;
      topbar.classList.add('is-tight');
      if (!over()) return;
      if ($('.toc a .s', topbar)) {
        topbar.classList.add('is-short');
        if (!over()) return;
        topbar.classList.remove('is-short');
      }
      topbar.classList.replace('is-tight', 'is-compact');
    }
    window.addEventListener('resize', debounce(fit, 100));
    fit();
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(fit);
    var tb = $('#toc-btn'), toc = $('#toc');
    if (tb && toc) {
      tb.addEventListener('click', function () {
        var open = toc.classList.toggle('is-open');
        tb.setAttribute('aria-expanded', open ? 'true' : 'false');
      });
      $$('a', toc).forEach(function (a) {
        a.addEventListener('click', function () {
          toc.classList.remove('is-open');
          tb.setAttribute('aria-expanded', 'false');
        });
      });
    }
  }

  /* ================= Shared: tabs ================= */

  function initTabs(list, onShow) {
    var tabs = $$('[role="tab"]', list);
    function select(t, focus) {
      tabs.forEach(function (x) {
        var on = x === t;
        x.setAttribute('aria-selected', on ? 'true' : 'false');
        x.tabIndex = on ? 0 : -1;
        var id = x.getAttribute('aria-controls');
        if (id) { var p = document.getElementById(id); if (p) p.hidden = !on; }
      });
      if (focus) t.focus();
      if (onShow) onShow(t);
    }
    tabs.forEach(function (t, i) {
      t.addEventListener('click', function () { select(t); });
      t.addEventListener('keydown', function (e) {
        var j = -1;
        if (e.key === 'ArrowRight') j = (i + 1) % tabs.length;
        if (e.key === 'ArrowLeft') j = (i - 1 + tabs.length) % tabs.length;
        if (e.key === 'Home') j = 0;
        if (e.key === 'End') j = tabs.length - 1;
        if (j >= 0) { e.preventDefault(); select(tabs[j], true); }
      });
    });
    var initial = tabs.filter(function (t) { return t.getAttribute('aria-selected') === 'true'; })[0] || tabs[0];
    select(initial);
    return select;
  }

  /* A segmented control: calls back with the pressed button's value attribute */
  function initSeg(root, attr, cb) {
    var bs = $$('button', root);
    bs.forEach(function (b) {
      b.addEventListener('click', function () {
        setPressed(bs, b);
        cb(b.getAttribute(attr), b);
      });
    });
    return bs;
  }

  global.RNLPage = {
    $: $, $$: $$, icon: icon, store: store, debounce: debounce, onVisible: onVisible, setPressed: setPressed,
    fill: fill, paint: paint, initTabs: initTabs, initSeg: initSeg
  };

  function boot() {
    paint(document);
    initTheme();
    initNav();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})(window);
