/* The page for agents: the copy buttons of the sentence and of the commands. */
(function () {
  'use strict';

  var DONE = '<zh>已复制</zh><en>Copied</en>';

  // the text to copy: the lines of a command block, or the text of the element
  function textOf(el) {
    if (el.children.length) {
      return Array.prototype.map.call(el.children, function (line) { return line.textContent; }).join('\n');
    }
    return el.textContent.trim();
  }

  // where the clipboard cannot be written, select the text so that Ctrl+C takes it
  function select(el) {
    var range = document.createRange();
    range.selectNodeContents(el);
    var s = window.getSelection();
    s.removeAllRanges();
    s.addRange(range);
  }

  Array.prototype.forEach.call(document.querySelectorAll('[data-copy]'), function (btn) {
    var el = document.getElementById(btn.getAttribute('data-copy'));
    var label = btn.querySelector('.copy-label');
    var text = label.textContent;
    var timer = null;
    function done() {
      btn.classList.add('is-done');
      label.textContent = DONE;
      clearTimeout(timer);
      timer = setTimeout(function () { btn.classList.remove('is-done'); label.textContent = text; }, 1800);
    }
    btn.addEventListener('click', function () {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(textOf(el)).then(done, function () { select(el); });
      } else {
        select(el);
      }
    });
  });
}());
