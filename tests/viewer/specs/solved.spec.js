// Netlists that only a solver draws: the global setup solved them with `rnl solve`, whose records
// place every object and route the wires, and the page opens the files solved as any other. The
// drawing is the solver's: no status of a level asks for a solver, every symbol and wire is drawn,
// and the routed nets have wires. Each test saves screenshots of the page under test-results, named
// after the project as well in the projects other than chromium.

import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "../viewer.js";

/** Where the screenshots go: test-results of this directory, which Playwright empties as a run starts. */
const RESULTS = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../test-results");

/** The path of the screenshot NAME.png, as solved-T3, in the project running: NAME-firefox.png in firefox. */
function screenshot(name) {
  const project = test.info().project.name;
  return path.join(RESULTS, project === "chromium" ? `${name}.png` : `${name}-${project}.png`);
}

/** The names of the objects of the level shown that a hit of `kind` stands for, each once, sorted. */
function names(page, kind) {
  return page.evaluate((kind) => {
    const doc = window.viewer.doc;
    const hits = doc.levels[window.viewer.level].hits.filter((h) => h.kind === kind);
    return [...new Set(hits.flatMap((h) => h.objects.map((o) => doc.objects[o].name)))].sort();
  }, kind);
}

/**
 * The symbols of the level shown that stand for a net, the supply and ground stubs and the ports:
 * for each net, by its name, how many boxes stand for it.
 */
function symbols(page) {
  return page.evaluate(() => {
    const doc = window.viewer.doc;
    const out = {};
    for (const h of doc.levels[window.viewer.level].hits.filter((h) => h.kind === "box")) {
      for (const o of h.objects.filter((o) => doc.objects[o].kind === "net")) {
        out[doc.objects[o].name] = (out[doc.objects[o].name] ?? 0) + 1;
      }
    }
    return out;
  });
}

/**
 * Expect the level shown, with the title `title`, to be drawn as the solver placed it: nothing on
 * it lacks a position or a solver, every symbol and wire of its hits is in the SVG, the instances
 * `instances` have symbols, the symbols that stand for a net are `nets`, so many for each net (each
 * stub and each port is one box, so that a port does not pass for the stubs of its net), and the
 * nets `wired` have wires. Then save a screenshot of the page as `shot`; see screenshot().
 */
async function drawn(page, viewer, { title, instances, nets, wired, shot }) {
  await expect(page.locator("#level-name")).toHaveText(title);
  await expect(page.locator("#empty")).toBeHidden();
  await expect(page.locator("#canvas > svg")).toHaveCount(1);
  await expect(page.locator("#status")).not.toHaveText(/Not drawn without a solver/);
  expect(await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].status)).toBeNull();
  const ids = await page.evaluate(() => window.viewer.doc.levels[window.viewer.level].hits.filter((h) => h.id).map((h) => h.id));
  for (const id of ids) await expect(page.locator(`#canvas > svg [data-id="${id}"]`).first()).toBeAttached();
  expect(await names(page, "box")).toEqual(expect.arrayContaining(instances));
  expect(await symbols(page)).toEqual(nets);
  expect(await names(page, "wire")).toEqual(expect.arrayContaining(wired));
  // each symbol is on the screen, as the level is fitted
  for (const name of instances) await viewer.where(name);
  await viewer.settled();
  await page.screenshot({ path: screenshot(shot) });
}

test("T3, the five-transistor OTA in its testbench, is drawn as the solver placed and routed it", async ({ page, viewer }) => {
  await viewer.open("T3-ota5t-solved");
  expect(await page.evaluate(() => window.viewer.doc.diagnostics.filter((d) => d.severity === "error"))).toEqual([]);
  await drawn(page, viewer, {
    title: "T3-ota5t.sp",
    instances: ["XU1", "VDD", "VB", "VIP", "VIN", "CL"],
    // the ground stubs of XU1, VDD, VB, VIP, VIN and CL; the supply stubs of XU1 and VDD
    nets: { 0: 6, vdd: 2 },
    wired: ["inp", "inn", "out", "vb"],
    shot: "solved-T3-top",
  });

  // the OTA's own level: the mirror on top, the pair below it, the tail at the bottom
  await page.locator("#tab-hierarchy").click();
  await page.locator("#index .index-row:not(.top)", { hasText: "ota5t" }).click();
  await page.locator("#tab-netlist").click();
  await drawn(page, viewer, {
    title: "ota5t",
    instances: ["M1", "M2", "M3", "M4", "M5"],
    // the supply stubs of M3 and M4 and the ground stub of M5, besides a port for each net
    nets: { vdd: 3, vss: 2, inp: 1, inn: 1, out: 1, vbias: 1 },
    wired: ["x", "tail", "out", "inp", "inn", "vbias"],
    shot: "solved-T3",
  });
  const middle = async (name) => (await viewer.where(name)).y;
  const [m1, m2, m3, m4, m5] = [await middle("M1"), await middle("M2"), await middle("M3"), await middle("M4"), await middle("M5")];
  expect(m1).toBeCloseTo(m2, 3);
  expect(m3).toBeCloseTo(m4, 3);
  expect(m3).toBeLessThan(m1);
  expect(m1).toBeLessThan(m5);
});

test("T2, the common-source stage, is drawn as the solver placed and routed it", async ({ page, viewer }) => {
  await viewer.open("T2-common-source-solved");
  expect(await page.evaluate(() => window.viewer.doc.diagnostics.filter((d) => d.severity === "error"))).toEqual([]);
  await drawn(page, viewer, {
    title: "T2-common-source.sp",
    instances: ["M1", "RD", "VDD", "VIN"],
    // the ground stubs of M1, VDD and VIN; the supply stubs of RD and VDD
    nets: { 0: 3, vdd: 2 },
    wired: ["in", "out"],
    shot: "solved-T2",
  });
});
