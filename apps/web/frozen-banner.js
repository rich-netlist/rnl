// The banner of the frozen viewer for 0.3 files (site/dist/viewer-0.3/, apps/web/freeze.mjs): a bar over the top
// of the page that says it is the old viewer and links the new one. It is a script of its own, not a part of the
// page of apps/viewer-ui, so that the old page, built from the tag it is frozen at, is not changed: freeze.mjs adds it
// to the built page. It follows the language the page shows (the `lang` of the document, which the page keeps up to
// date, and its `language-changed` event), with the texts of the seven languages of the interface and English.
(function () {
  "use strict";

  // [the text before the link, the text of the link, the text after it], by the code of the language of the registry
  var TEXTS = {
    en: ["This is the old viewer, for RNL 0.3 files. For RNL 0.4, use ", "the new viewer", "."],
    de: ["Dies ist der alte Viewer für RNL-0.3-Dateien. Für RNL 0.4 verwenden Sie ", "den neuen Viewer", "."],
    es: ["Este es el visor antiguo, para archivos RNL 0.3. Para RNL 0.4, use ", "el visor nuevo", "."],
    fr: ["Voici l’ancienne visionneuse, pour les fichiers RNL 0.3. Pour RNL 0.4, utilisez ", "la nouvelle visionneuse", "."],
    ja: ["これは RNL 0.3 ファイル用の旧版ビューアです。RNL 0.4 には", "新版ビューア", "をご利用ください。"],
    ko: ["RNL 0.3 파일용 이전 버전 뷰어입니다. RNL 0.4는 ", "새 뷰어", "를 사용하세요."],
    zh: ["这是读取 RNL 0.3 文件的旧版查看器。RNL 0.4 请使用", "新版查看器", "。"],
    "zh-hant": ["這是讀取 RNL 0.3 檔案的舊版檢視器。RNL 0.4 請使用", "新版檢視器", "。"],
  };

  var script = document.currentScript;
  var href = (script && script.getAttribute("data-href")) || "../viewer/";

  /** The texts for a language tag such as "de", "zh", "zh-hant" or "pt-BR" (English for one the viewer does not have). */
  function textsFor(tag) {
    tag = String(tag || "en").toLowerCase();
    if (TEXTS[tag]) return TEXTS[tag];
    if (tag.indexOf("zh") === 0) return /hant|tw|hk|mo/.test(tag) ? TEXTS["zh-hant"] : TEXTS.zh;
    return TEXTS[tag.split("-")[0]] || TEXTS.en;
  }

  var style = document.createElement("style");
  style.textContent =
    "#frozen-banner{position:fixed;top:0;left:0;right:0;z-index:30;box-sizing:border-box;padding:5px 12px;text-align:center;" +
    "font:13px/1.4 system-ui,-apple-system,'Segoe UI',sans-serif;background:#fff1c9;color:#4a3a00;border-bottom:1px solid #e3cf8a}" +
    "#frozen-banner a{color:inherit;font-weight:600;text-decoration:underline}" +
    "@media (prefers-color-scheme:dark){#frozen-banner{background:#3b3208;color:#f3e4a0;border-bottom-color:#675a1c}}" +
    "body.has-frozen-banner{padding-top:var(--frozen-banner-height,28px)}";
  document.head.appendChild(style);

  var bar = document.createElement("div");
  bar.id = "frozen-banner";
  bar.setAttribute("role", "note");

  function render() {
    var parts = textsFor(document.documentElement.lang);
    var link = document.createElement("a");
    link.href = href;
    link.textContent = parts[1];
    bar.textContent = "";
    bar.appendChild(document.createTextNode(parts[0]));
    bar.appendChild(link);
    bar.appendChild(document.createTextNode(parts[2]));
    // (the page below it is as high as what is left: a bar that wraps on a narrow screen takes more)
    document.body.style.setProperty("--frozen-banner-height", bar.offsetHeight + "px");
  }

  document.body.classList.add("has-frozen-banner");
  document.body.appendChild(bar);
  render();
  window.addEventListener("language-changed", render);
  window.addEventListener("resize", render);
})();
