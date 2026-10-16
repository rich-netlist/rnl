// Tests of package-file.mjs: node --test site/gallery/package-file.test.mjs

import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { classify, params, readPackage, resolve, scan, terminalNames } from "./package-file.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

test("members split at the semicolons of their own record only", () => {
  const text = [
    "/* a file */",
    "@RNL {",
    '  $note = @DEFINE-STYLE(textcolor="#555", font="a;b") {};',
    "  /* the rule; with a semicolon */",
    '  # @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RES;',
    '  $S = @DEFINE-SYMBOL { @LINE {(0, 0) (1, 1)}; ? @TEXT("say ""a;b""", key="k") }',
    "}",
  ].join("\n");
  const { records, comments } = scan(text);
  assert.equal(records.length, 1);
  assert.deepEqual(
    records[0].members.map((m) => m.text.slice(0, 12)),
    ["$note = @DEF", "# @SELECT-IN", "$S = @DEFINE"],
  );
  assert.deepEqual(records[0].members[1].docs, ["/* the rule; with a semicolon */"]);
  assert.equal(comments.length, 2);
});

test("the header record gives the package, its references and its aliases", () => {
  const p = readPackage(
    [
      "/* rnl-x: a test.",
      "   Second line. */",
      '@RNL(type="header") {',
      '  @VERSION("0.3");',
      '  @PACKAGE(name="rnl-x", version="0.3");',
      '  @USE-PACKAGE(name="rnl-y", version="0.3");',
      '  @ALIAS("@位于", "@PLACE");',
      '  @ALIAS("@a""b", "@C")',
      "}",
    ].join("\r\n"),
  );
  assert.deepEqual(p.header.language, "0.3");
  assert.equal(p.header.name, "rnl-x");
  assert.equal(p.header.version, "0.3");
  assert.deepEqual(p.header.references, [{ name: "rnl-y", version: "0.3" }]);
  assert.deepEqual(p.header.aliases, [["@位于", "@PLACE"], ['@a"b', "@C"]]);
  assert.equal(p.description, "rnl-x: a test.\nSecond line.");
  assert.ok(!p.text.includes("\r"));
  assert.deepEqual(p.statements, []);
});

test("statements are sorted by kind", () => {
  const kinds = (s) => classify(s).kind;
  assert.equal(kinds("$R = @DEFINE-SYMBOL(box=[-1, -2, 1, 2]) {(0, 2) @PIN(terminal=1)}"), "symbol");
  assert.equal(kinds("%BODY = @DEFINE-SYMBOL {}"), "symbol");
  assert.equal(kinds('$base = @DEFINE-STYLE(linecolor="#1d1d1f") {}'), "style");
  assert.equal(kinds('$GND = @DEFINE(roles=[["p", ["net", "terminal"]]]) {}'), "definition");
  assert.equal(kinds('$NETS = # @SELECT-NET(name="0")'), "set");
  assert.equal(kinds("$EMPTY = {}"), "set");
  assert.equal(kinds("$GND_STUB @ATTACH $GND_FILL_SET"), "apply");
  const rule = classify('# @SELECT-INSTANCE(type="pmos") => @SET-SYMBOL $PMOS4');
  assert.deepEqual([rule.kind, rule.action, rule.target, rule.select], ["rule", "symbol", "$PMOS4", '# @SELECT-INSTANCE(type="pmos")']);
  const edit = classify("$GROUND_NET_SET @REMOVE-FROM $WIRE_NET_SET");
  assert.deepEqual([edit.kind, edit.action, edit.target], ["rule", "remove-from", "$WIRE_NET_SET"]);
  assert.equal(classify('# @SELECT-INSTANCE => @SET-STYLE $base').action, "style");
  assert.equal(classify('# @SELECT-INSTANCE => @TEXT("", key="name")').action, "text");
  assert.deepEqual(classify("$S = @DEFINE-SYMBOL(box=[-0.5, -1, 0.5, 1]) {}").box, [-0.5, -1, 0.5, 1]);
});

test("a symbol's pins and labels include those of the parts it draws", () => {
  const p = readPackage(
    [
      '@RNL(type="header") { @VERSION("0.2"); @PACKAGE(name="t", version="0.2")}',
      "@RNL {",
      "  %CH = @DEFINE-SYMBOL {",
      '    (-3, 0) @PIN(terminal=2, direction="left");',
      '    ? @TEXT("${name}", key="name") => @PLACE(0.5, 1)',
      "  };",
      "  $N = @DEFINE-SYMBOL(box=[-3, -2, 0, 2]) {",
      "    %CH;",
      '    (0, 2) @PIN(terminal=1, direction="up");',
      '    (0, -2) @PIN(terminal=["a", "b"], label="AB", direction="down")',
      "  }",
      "}",
    ].join("\n"),
  );
  const symbols = new Map(p.statements.map((s) => [s.name, s]));
  const n = symbols.get("$N");
  assert.deepEqual(n.parts, ["%CH"]);
  assert.equal(n.line, 7);
  assert.equal(n.endLine, 11);
  const { pins, labels } = resolve(n, symbols);
  assert.deepEqual(
    pins.map((x) => [x.terminal, x.at, x.direction, x.label]),
    [
      [2, [-3, 0], "left", undefined],
      [1, [0, 2], "up", undefined],
      [["a", "b"], [0, -2], "down", "AB"],
    ],
  );
  assert.deepEqual(labels, [{ key: "name", text: "${name}" }]);
});

test("terminal names come only from a comment that starts with them", () => {
  assert.deepEqual(terminalNames(["a b y (vp) (vn); from Analog Canvas and-gate"]), ["a", "b", "y", "vp", "vn"]);
  assert.deepEqual(terminalNames(["d g s b; from Analog Canvas nmos"]), ["d", "g", "s", "b"]);
  assert.equal(terminalNames(["MOS: terminals d g s b = 1 2 3 4 (SPICE order). NMOS drain up"]), null);
  assert.equal(terminalNames(["a generic op-amp: terminals in+ in- out = 1 2 3"]), null);
  assert.equal(terminalNames([]), null);
});

test("parameters keep brackets and quoted commas", () => {
  assert.deepEqual(params('name="a, b", roles=[["p", "point"]], gap=2'), { name: "a, b", roles: '[["p", "point"]]', gap: "2" });
});

test("every package file in libraries/ reads as a package", () => {
  const files = [];
  (function walk(dir) {
    for (const e of readdirSync(dir)) {
      const path = join(dir, e);
      if (statSync(path).isDirectory()) walk(path);
      else if (e.endsWith(".rnl")) files.push(path);
    }
  })(join(root, "libraries"));
  assert.ok(files.length > 0);
  for (const file of files) {
    const p = readPackage(readFileSync(file, "utf8"));
    const lines = p.text.split("\n").length;
    assert.ok(p.header.name && p.header.version && p.header.language, `${file}: a package name, version and language`);
    for (const s of p.statements) assert.ok(s.line >= 1 && s.endLine >= s.line && s.endLine <= lines, `${file}:${s.line}`);
  }
});
