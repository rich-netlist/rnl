/* Shared by the site and the downloadable standard pages; native details works without JavaScript. */
(function () {
  'use strict';
  document.querySelectorAll('.language-menu').forEach(function (menu) {
    var summary = menu.querySelector('summary');
    document.addEventListener('click', function (event) {
      if (!menu.contains(event.target)) menu.open = false;
    });
    document.addEventListener('keydown', function (event) {
      if (event.key === 'Escape' && menu.open) {
        event.preventDefault();
        menu.open = false;
        summary.focus();
      }
    });
    menu.querySelectorAll('[data-lang-link]').forEach(function (link) {
      link.addEventListener('click', function () {
        try { localStorage.setItem('rnl-site-language', link.dataset.language); } catch (error) { /* storage unavailable */ }
      });
    });
  });
}());
