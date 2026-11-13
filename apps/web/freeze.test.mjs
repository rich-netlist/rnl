import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runInNewContext } from "node:vm";
import { freeze, withBanner } from "./freeze.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const page = "<!doctype html><html><head></head><body class=\"starting\">\n<script src=\"main.js\"></script>\n</body></html>\n";

test("the script of the banner goes before the end of the body, once", () => {
  const once = withBanner(page);
  assert.match(once, /<script src="frozen-banner\.js" data-href="\.\.\/viewer\/"><\/script>\n<\/body>/);
  assert.equal(withBanner(once), once);
  assert.ok(once.indexOf("main.js") < once.indexOf("frozen-banner.js"), "the page's own scripts come first");
  assert.match(withBanner(page, 'a"b&c'), /data-href="a&quot;b&amp;c"/);
  assert.throws(() => withBanner("<html></html>"), /no end of its body/);
});

test("freezing a built viewer writes the banner beside its page", () => {
  const dist = mkdtempSync(join(tmpdir(), "rnl-freeze-"));
  try {
    writeFileSync(join(dist, "index.html"), page);
    freeze(dist, "../viewer/");
    assert.ok(existsSync(join(dist, "frozen-banner.js")));
    assert.match(readFileSync(join(dist, "index.html"), "utf8"), /frozen-banner\.js/);
  } finally {
    rmSync(dist, { recursive: true, force: true });
  }
});

/** The banner's script run against a bare stand-in for a page: the document, and what its bar says and links. */
function run(lang) {
  const listeners = {};
  const element = (tag) => ({
    tag, children: [], style: { setProperty() {} }, attrs: {}, offsetHeight: 28, classList: { add() {} },
    appendChild(child) { this.children.push(child); return child; },
    setAttribute(name, value) { this.attrs[name] = value; },
    getAttribute(name) { return this.attrs[name] ?? null; },
    set textContent(value) { this.text = value; if (value === "") this.children = []; },
  });
  const body = element("body");
  const document = {
    documentElement: { lang },
    head: element("head"),
    body,
    currentScript: { getAttribute: (name) => (name === "data-href" ? "../viewer/" : null) },
    createElement: element,
    createTextNode: (text) => ({ text }),
  };
  const window = { addEventListener: (type, fn) => (listeners[type] = fn) };
  runInNewContext(readFileSync(join(here, "frozen-banner.js"), "utf8"), { document, window });
  const bar = body.children[0];
  return { bar, listeners, document, text: () => bar.children.map((c) => c.text).join("") };
}

test("the banner says it is the old viewer, in each language of the interface, and links the new viewer", () => {
  const expectations = { en: "RNL 0.3", de: "alte Viewer", es: "visor antiguo", fr: "ancienne visionneuse", ja: "旧版", ko: "이전 버전", zh: "旧版查看器", "zh-hant": "舊版檢視器" };
  for (const [lang, word] of Object.entries(expectations)) {
    const { bar, text } = run(lang);
    assert.ok(text().includes(word), `${lang}: ${text()}`);
    assert.ok(text().includes("0.4"), lang);
    const link = bar.children.find((c) => c.tag === "a");
    assert.equal(link.href, "../viewer/", lang);
    assert.ok(link.text.length > 0, lang);
  }
});

test("the banner follows the language of the page, and a language it does not have is English", () => {
  const { document, listeners, text, bar } = run("en");
  document.documentElement.lang = "zh-Hant";
  listeners["language-changed"]();
  assert.ok(text().includes("舊版檢視器"), text());
  document.documentElement.lang = "zh";
  listeners["language-changed"]();
  assert.ok(text().includes("旧版查看器"), text());
  document.documentElement.lang = "pt-BR";
  listeners["language-changed"]();
  assert.ok(text().startsWith("This is the old viewer"), text());
  assert.equal(bar.children.filter((c) => c.tag === "a").length, 1, "the link is replaced, not added to");
});
