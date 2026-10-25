// Tests of public-packages.mjs: node --test site/gallery/public-packages.test.mjs

import assert from "node:assert/strict";
import { test } from "node:test";
import { withTranslatedSummaries } from "./public-packages.mjs";

const catalog = "https://example.com/rnl-packages/catalog.json";
const files = {
  "https://example.com/rnl-packages/summaries/zh.json": { "Symbols in two words.": "两个词的符号。", "Styles.": "" },
  "https://example.com/rnl-packages/summaries/ja.json": { "Symbols in two words.": "記号。" },
};
const fetcher = async (url) => url in files ? new Response(JSON.stringify(files[url])) : new Response("", { status: 404 });

test("summaries: translations from beside the catalog, after a record's own", async () => {
  const packages = [
    { name: "a", summary: { en: "Symbols in two\n  words." } },
    { name: "b", summary: { zh: "自己的。", en: "Symbols in two words." } },
    { name: "c", summary: { en: "Styles." } },
    { name: "d", summary: { en: "Not translated." } },
  ];
  const [a, b, c, d] = await withTranslatedSummaries(packages, catalog, ["en", "zh", "ja", "ko"], fetcher);
  assert.deepEqual(a.summary, { zh: "两个词的符号。", ja: "記号。", en: "Symbols in two\n  words." });
  assert.deepEqual(b.summary, { zh: "自己的。", ja: "記号。", en: "Symbols in two words." });
  assert.deepEqual(c.summary, { en: "Styles." }); // an empty translation shows the English
  assert.equal(d, packages[3]);
});

test("summaries: a catalog without translations, or one that cannot be read, shows the English", async () => {
  const packages = [{ name: "a", summary: { en: "Symbols in two words." } }];
  assert.deepEqual(await withTranslatedSummaries(packages, catalog, ["en", "ko"], fetcher), packages);
  const failing = async () => { throw new Error("offline"); };
  const warn = console.warn;
  console.warn = () => {};
  try {
    assert.deepEqual(await withTranslatedSummaries(packages, catalog, ["en", "zh"], failing), packages);
  } finally {
    console.warn = warn;
  }
});
