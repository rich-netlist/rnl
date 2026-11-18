// A junction dot the solver writes stands for the net of the wires it joins (#634): its box hit carries
// [usage, net], so that selecting the net lights its dots up with its wires, and selecting a dot lights
// the dot alone. The five-transistor OTA's own level has six of them: one on out, two on nb, one on tail and two on x.

import { expect, test } from "../viewer.js";

const NETS = { out: 1, nb: 2, tail: 1, x: 2 };

/** The junction dots of the level shown: [the group id of the dot, its usage's name, its net's name] each, by position in the file. */
function dots(page) {
  return page.evaluate(() => {
    const doc = window.viewer.doc;
    return doc.levels[window.viewer.level].hits
      .filter((h) => h.kind === "box" && h.id && h.objects.length === 2 && doc.objects[h.objects[0]].kind === "usage" && doc.objects[h.objects[1]].kind === "net")
      .map((h) => [h.id, doc.objects[h.objects[0]].name, doc.objects[h.objects[1]].name])
      .filter(([, usage]) => /^@ota5t\.scs:\d+:\d+$/.test(usage));
  });
}

for (const platform of ["web", "desktop"]) {
  test(`${platform}: selecting a net lights the junction dots on it, and selecting a dot lights it alone`, async ({ page, viewer }) => {
    await viewer.open("ota5t-solved", { platform });
    await page.locator("#tab-hierarchy").click();
    await page.locator("#index .index-row:not(.top)", { hasText: "ota5t" }).click();
    await page.locator("#tab-netlist").click();
    await expect(page.locator("#level-name")).toHaveText("ota5t");
    const all = await dots(page);
    const count = {};
    for (const [, , net] of all) count[net] = (count[net] ?? 0) + 1;
    expect(count).toEqual(NETS);
    const idsOf = (net) => all.filter(([, , n]) => n === net).map(([id]) => id);

    // a net's wire: its dots are among what is lit, and the dots of the other nets are not
    for (const net of Object.keys(NETS)) {
      await viewer.click(net, "wire");
      await viewer.selected(`net ${net} in ota5t`);
      const lit = await viewer.selectedGroups();
      for (const id of idsOf(net)) expect(lit).toContain(id);
      for (const other of Object.keys(NETS).filter((n) => n !== net)) for (const id of idsOf(other)) expect(lit).not.toContain(id);
    }

    // a dot: it is selected as the usage it is, and only it is lit, not the wires of its net nor the net's other dots
    const [[id, usage, net]] = all.filter(([, , n]) => n === "x");
    // (the click is on the dot and on the wire under it: the list that opens has the dot's usage and the net)
    await viewer.click(usage);
    await expect(page.locator("#pick-list")).toBeVisible();
    await expect(page.locator("#pick-list .pick-row .kind")).toHaveText(["net", "usage"]);
    await expect(page.locator("#pick-list .pick-row .name")).toHaveText([net, usage]);
    await page.locator("#pick-list .pick-row", { hasText: usage }).click();
    await viewer.selected(`usage ${usage} in ota5t`);
    expect(await viewer.selectedGroups()).toEqual([id]);
    // and the net it stands for is still a click away, which lights all its dots again
    await viewer.click(net, "wire");
    await viewer.selected(`net ${net} in ota5t`);
    expect(await viewer.selectedGroups()).toEqual(expect.arrayContaining(idsOf(net)));
  });
}
