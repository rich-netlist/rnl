/* RNL introduction page · interactions of the main page */
(function () {
  'use strict';

  var S = window.RNLSchem, SC = window.RNLScenes, C = window.RNLCode, PG = window.RNLPage;
  var $ = PG.$, $$ = PG.$$, icon = PG.icon, onVisible = PG.onVisible, setPressed = PG.setPressed, fill = PG.fill, debounce = PG.debounce;
  var REDUCE = S.REDUCE;

  /* ================= Hero: the logo's circuit ================= */

  function initLogoViewer() {
    var button = $('#hero-logo'), panel = $('#hero-logo-panel'), closeButton = $('#hero-logo-close'), brand = $('#hero-brand');
    if (!button || !panel || !closeButton || !brand) return;
    var frame = null, viewerOrigin = null, ready = false, needsFit = false;
    var mq = window.matchMedia('(prefers-color-scheme: dark)');
    function theme() {
      var chosen = document.documentElement.getAttribute('data-theme');
      return chosen === 'dark' || chosen === 'light' ? chosen : mq.matches ? 'dark' : 'light';
    }
    function syncTheme() {
      if (frame && ready) frame.contentWindow.postMessage({ rnl: 1, type: 'background', value: theme() }, viewerOrigin);
    }
    function fitIfNeeded() {
      if (!frame || !ready || panel.hidden || !needsFit) return;
      frame.contentWindow.postMessage({ rnl: 1, type: 'fit' }, viewerOrigin);
      needsFit = false;
    }
    function close() {
      panel.hidden = true;
      brand.classList.remove('is-viewing');
      button.setAttribute('aria-expanded', 'false');
      // Let the hidden iframe release focus before returning it to the restored wordmark.
      requestAnimationFrame(function () { if (panel.hidden) button.focus({ preventScroll: true }); });
    }
    function escape(e) {
      if (e.key !== 'Escape' || panel.hidden) return;
      e.preventDefault();
      e.stopPropagation();
      close();
    }
    button.addEventListener('click', function () {
      if (!panel.hidden) { close(); return; }
      panel.hidden = false;
      brand.classList.add('is-viewing');
      button.setAttribute('aria-expanded', 'true');
      if (!frame) {
        var url = new URL(panel.getAttribute('data-viewer-url'), document.baseURI);
        url.search = new URLSearchParams({ embed: '1', url: 'samples/brand/rnl-logo.sp', background: theme(), grid: 'off' });
        if (location.origin !== 'null') url.searchParams.set('origin', location.origin);
        viewerOrigin = url.origin;
        frame = document.createElement('iframe');
        frame.title = panel.getAttribute('aria-label');
        frame.addEventListener('load', function () {
          // The published homepage and viewer are same-origin. Capture Escape inside the iframe
          // too; keyboard events do not bubble between browsing contexts.
          var child = frame.contentDocument;
          if (child) child.addEventListener('keydown', escape, true);
        });
        frame.src = url.href;
        panel.appendChild(frame);
      } else { syncTheme(); fitIfNeeded(); }
      closeButton.focus({ preventScroll: true });
    });
    closeButton.addEventListener('click', close);
    document.addEventListener('keydown', escape, true);
    window.addEventListener('message', function (e) {
      if (!frame || e.source !== frame.contentWindow || e.origin !== viewerOrigin || e.data?.rnl !== 1) return;
      if (e.data.type === 'ready') { ready = true; syncTheme(); fitIfNeeded(); }
    });
    new MutationObserver(syncTheme).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    new ResizeObserver(function () {
      needsFit = true;
      fitIfNeeded();
    }).observe(brand);
    mq.addEventListener('change', syncTheme);
  }

  /* ================= 01 Thumbnails for the workarounds ================= */

  function initFixes() {
    var svg = $('#fix-air');
    if (!svg) return;
    new S.View(svg, { box: SC.OTA_BOX, grid: false }).setScene(SC.otaScene(0));
  }

  /* ================= 05 Vision: the central file and its links ================= */

  function initHub() {
    var hub = $('#hub'), svg = $('#hub-wires'), core = $('#hub-core');
    if (!hub) return;
    $('#hub-lines').innerHTML = C.hlFile([
      '* Five-transistor OTA',
      'M1 x inp tail vss nch W=2u L=0.5u',
      'M2 out inn tail vss nch W=2u L=0.5u',
      '* @RNL {',
      '*   %dp1 = #M1 #M2 @AS $DIFFERENTIAL_PAIR;',
      '*   %cm1 = #M3 #M4 @AS $CURRENT_MIRROR',
      '* }'
    ]).map(function (h) { return '<div>' + h.html + '</div>'; }).join('');
    var nodes = $$('.hub-node', hub);
    function draw() {
      while (svg.firstChild) svg.removeChild(svg.firstChild);
      if (window.getComputedStyle(svg).display === 'none') return;
      var hb = hub.getBoundingClientRect(), cb = core.getBoundingClientRect();
      nodes.forEach(function (n, i) {
        var nb = n.getBoundingClientRect();
        var above = nb.bottom <= cb.top;
        var x1 = Math.round(nb.left - hb.left + nb.width / 2);
        /* from the file's edge to the box's edge: the boxes above are fed from the left and the right of the top edge */
        var x0 = above ? Math.round(cb.left - hb.left + cb.width * (x1 + hb.left < cb.left + cb.width / 2 ? 0.3 : 0.7)) : x1;
        var y0 = Math.round(above ? cb.top - hb.top : cb.bottom - hb.top);
        var y1 = Math.round(above ? nb.bottom - hb.top : nb.top - hb.top);
        var ym = Math.round((y0 + y1) / 2);
        var d = 'M' + x0 + ' ' + y0 + ' V' + ym + ' H' + x1 + ' V' + y1;
        S.el('path', { d: d }, svg);
        S.el('circle', { cx: x0, cy: y0, r: 3.2, class: 'end' }, svg);
        S.el('circle', { cx: x1, cy: y1, r: 3.2, class: 'end' }, svg);
        if (!REDUCE) {
          var c = S.el('circle', { r: 3.4, class: 'pulse' }, svg);
          S.el('animateMotion', { dur: '2.8s', repeatCount: 'indefinite', begin: (i * 0.45).toFixed(2) + 's', path: d }, c);
        }
      });
    }
    draw();
    window.addEventListener('resize', debounce(draw, 150));
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(draw);
  }

  /* ================= 04 Scenarios ================= */

  function initScenes() {
    var list = $('#sc-tabs');
    if (!list) return;
    var made = {};
    function thumbs(id) {
      if (made[id]) return;
      made[id] = true;
      if (id === 't-ai') {
        new S.View($('#ai-thumb'), { box: SC.OTA_BOX, grid: false }).setScene(SC.otaScene(2));
      }
    }
    PG.initTabs(list, function (t) { thumbs(t.id); });
  }

  /* ================= Startup ================= */

  function boot() {
    initLogoViewer();
    initFixes();
    initHub();
    if (window.RNLOta) window.RNLOta.init();
    initScenes();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();
