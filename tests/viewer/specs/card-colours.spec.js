// The pages shown in place of a drawing, the module of Verilog-A (module-view.js) and the entry page (entry-page.js),
// lie on the drawing's background, which View › Background sets apart from the theme of the window: their text is dark
// on a light background and light on a dark one, whichever the theme.
import { expect, test } from "../viewer.js";

const DARK_INK = "rgb(29, 29, 31)";
const LIGHT_INK = "rgb(232, 232, 234)";

for (const scheme of ["light", "dark"]) {
  test(`the module and entry pages follow the drawing's background in the ${scheme} theme`, async ({ page, viewer }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await viewer.open("04-hierarchy");
    // the cards as the pages fill them, with a heading and a note
    await page.evaluate(() => {
      for (const id of ["module-card", "entry-card"]) {
        const card = document.getElementById(id);
        card.innerHTML = '<div class="module-heading entry-heading"><span class="module-name">top_va</span></div><p class="module-note entry-note">note</p>';
        card.hidden = false;
      }
    });
    const colours = () => page.evaluate(() => Object.fromEntries(["module-card", "entry-card"].map((id) => {
      const card = document.getElementById(id);
      return [id, { heading: getComputedStyle(card.querySelector(".module-name")).color, note: getComputedStyle(card.querySelector("p")).color }];
    })));
    await page.evaluate(() => window.viewer.command("background-light"));
    for (const c of Object.values(await colours())) {
      expect(c.heading).toBe(DARK_INK);
      expect(c.note).toBe("rgb(107, 107, 112)");
    }
    await page.evaluate(() => window.viewer.command("background-dark"));
    for (const c of Object.values(await colours())) {
      expect(c.heading).toBe(LIGHT_INK);
      expect(c.note).toBe("rgb(154, 154, 162)");
    }
  });
}
