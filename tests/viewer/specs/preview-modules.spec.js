// The modules of Verilog-A in the preview of a solve (solve.js `keepOutline`, level-tree.js `previewModules`): the preview
// is a Document of the whole document, which has no modules, and it keeps the outline of the reading the solve started
// from, as a shell that reads the file shown alone gives it. An instance that names a module opens it in place, as an
// instance of a subcircuit opens its level, and the preview stays: the way back leads to it.

import { expect, test } from "../viewer.js";

const DESKTOP = { platform: "desktop" };

/**
 * The outline of the netlist of va-top (tests/viewer/inputs/va-top) as a shell that reads the file shown alone gives
 * it with its reading, by the files of its Document `doc`: the netlist's file level, then each Verilog-A file and its
 * modules.
 */
function outlineOf(doc) {
  const file = (name) => doc.files.findIndex((f) => f.path.endsWith(name));
  const [top, a, b] = [file("top.scs"), file("a.va"), file("b.va")];
  const level = (kind, title, at, extra = {}) => ({ kind, title, file: at, lines: null, parent: null, circuit: kind !== "module" && at === top,
    ports: [], instances: 0, calls: [], version: null, packages: [], ...extra });
  return {
    dialect: "spectre",
    files: doc.files.map((f, k) => ({ path: f.path, name: f.name, format: null, default_format: null, includes: [],
      levels: k === top ? [0] : k === a ? [1, 2, 3] : [4, 5], text_hash: "0000000000000000" })),
    levels: [
      level("file", doc.levels[0].title, top, { instances: 1, calls: [{ master: "top_va", level: 2, count: 1, instances: ["I1"], lines: [5] }] }),
      level("file", "a.va", a),
      level("module", "top_va", a, { lines: [1, 9], ports: ["x", "y", "g"], instances: 4, calls: [
        { master: "gain", level: 3, count: 2, instances: ["u1", "u4"], lines: [4, 7] },
        { master: "amp", level: 5, count: 1, instances: ["u2"], lines: [5] },
        { master: "ghost", level: null, count: 1 },
      ] }),
      level("module", "gain", a, { lines: [10, 13], ports: ["a", "b"] }),
      level("file", "b.va", b),
      level("module", "amp", b, { lines: [1, 4], ports: ["inp", "outp"] }),
    ],
    roots: [],
    globals: [],
    diagnostics: [],
    missing: [],
  };
}

/** Open va-top with the outline a reading of the file shown alone has, and solve it: the preview is shown. */
async function solved(page, viewer) {
  await page.route("**/fixtures/va-top.json", async (route) => {
    const doc = await (await route.fetch()).json();
    await route.fulfill({ json: { ...doc, outline: outlineOf(doc) } });
  });
  await viewer.open("va-top", DESKTOP);
  await page.evaluate(() => window.viewer.command("solve"));
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
  await expect(page.locator("#preview-badge")).toBeVisible();
}

/** What the page shows: the module shown, the kind of the level and the path's names. */
const shown = (page) => page.evaluate(() => ({
  module: document.getElementById("module-card").hidden ? null : document.querySelector("#module-card .module-name").textContent,
  kind: document.getElementById("level-kind").textContent,
  path: [...document.querySelectorAll("#level-path .crumb")].map((c) => c.querySelector(".name, #level-name")?.textContent),
}));

test("a double click on an instance of a module in the preview opens it in place, and the way back is the preview", async ({ page, viewer }) => {
  await solved(page, viewer);
  const top = await page.locator("#level-name").textContent();
  await viewer.click("I1");
  await expect(page.locator("#selection")).toContainText("I1");
  const p = await viewer.where("I1");
  await page.mouse.dblclick(p.x, p.y);
  await expect(page.locator("#module-card")).toBeVisible();
  expect(await shown(page)).toEqual({ module: "top_va", kind: "Verilog-A module", path: [top, "top_va"] });
  // the preview stays, its instances listed
  await expect(page.locator("#preview-badge")).toBeVisible();
  await expect(page.locator("#module-card .module-instance .inst")).toHaveText(["u1", "u4", "u2", "1×"]);
  // an instance of the module opens the module it names, in place too
  await page.locator("#module-card .module-instance", { hasText: "u2" }).dblclick();
  expect(await shown(page)).toEqual({ module: "amp", kind: "Verilog-A module", path: [top, "top_va", "amp"] });
  // back, to the module, then to the level of the preview, which is still the preview
  await page.keyboard.press("Backspace");
  expect(await shown(page)).toMatchObject({ module: "top_va", path: [top, "top_va"] });
  await page.keyboard.press("Backspace");
  await expect(page.locator("#module-card")).toBeHidden();
  await expect(page.locator("#level-name")).toHaveText(top);
  await expect(page.locator("#preview-badge")).toBeVisible();
  expect(await page.evaluate(() => document.body.classList.contains("preview"))).toBe(true);
  expect(await viewer.calls("build")).toHaveLength(1);
});

test("the hierarchy of the preview lists the modules, and a module's row opens it in place", async ({ page, viewer }) => {
  await solved(page, viewer);
  await page.locator("#tab-hierarchy").click();
  const rows = page.locator("#tree .tree-row");
  await expect(rows.nth(1)).toHaveText("I1top_vaVerilog-A");
  await rows.nth(1).click();
  await expect(page.locator("#module-card")).toBeVisible();
  await expect(page.locator("#module-card .module-name")).toHaveText("top_va");
  await expect(page.locator("#preview-badge")).toBeVisible();
});
