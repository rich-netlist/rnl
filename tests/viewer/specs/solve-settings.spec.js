import { expect, test } from "../viewer.js";
const dialog = (page) => page.locator("dialog.dialog");
const field = (page, label) => dialog(page).getByLabel(label, { exact: true });
async function open(page) {
  await page.evaluate(() => window.viewer.command("solve-settings"));
  await expect(dialog(page).locator(".dialog-title")).toHaveText("Solver Settings");
}
async function ok(page) {
  await dialog(page).getByRole("button", { name: "OK", exact: true }).click();
  await expect(dialog(page)).toHaveCount(0);
}
async function solved(page, viewer) {
  const before = (await viewer.calls("solve")).length;
  await page.evaluate(() => window.viewer.command("solve"));
  await expect.poll(async () => (await viewer.calls("solve")).length).toBe(before + 1);
  await page.waitForFunction(() => document.body.classList.contains("preview") && !document.body.classList.contains("solving"));
  return (await viewer.calls("solve")).at(-1)[1];
}

test("the web has no Solve settings", async ({ page, viewer }) => {
  await viewer.open("T4-miller-conflicts");
  await page.evaluate(() => window.viewer.command("solve-settings"));
  await expect(dialog(page)).toHaveCount(0);
  expect(await viewer.calls("solverDefaults")).toEqual([]);
});

test.describe("desktop scalar rules and TOML selection", () => {
  test.beforeEach(async ({ viewer }) => { await viewer.open("T4-miller-conflicts", { platform: "desktop" }); });

  test("defaults and exact decimal text survive settings and solve IPC", async ({ page, viewer }) => {
    await open(page);
    await expect(field(page, "Tolerance")).toHaveValue("0.000001");
    await expect(dialog(page).locator(".objective")).toHaveCount(0);
    await field(page, "Tolerance").fill("0.10000000000000001");
    await field(page, "Soft router cells").fill("12345");
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({ tolerance: "0.10000000000000001", budget: { soft_expansions: "12345" } });
    expect(await solved(page, viewer)).toEqual({ options: "tolerance=0.10000000000000001;budget=soft_expansions:12345", hintsForLow: false });
  });

  test("a selected config path and its errors reach the host", async ({ page, viewer }) => {
    await page.evaluate(() => { window.fakeShell.configPick = "C:/configs/custom.toml"; });
    await open(page);
    await dialog(page).getByRole("button", { name: "Choose File…" }).click();
    await expect(field(page, "Configuration file")).toHaveValue("C:/configs/custom.toml");
    await page.evaluate(() => { window.fakeShell.optionsError = "solver config C:/configs/custom.toml: line 4: unknown objective"; });
    await dialog(page).getByRole("button", { name: "OK", exact: true }).click();
    await expect(dialog(page).locator(".dialog-error")).toContainText("line 4");
    await page.evaluate(() => { window.fakeShell.optionsError = null; });
    await ok(page);
    expect((await viewer.calls("checkSolverOptions")).at(-1)).toEqual(["", "C:/configs/custom.toml"]);
    expect(await solved(page, viewer)).toEqual({ options: "", hintsForLow: false, solverConfig: "C:/configs/custom.toml" });
  });

  test("old objective preferences are discarded once while scalar preferences survive", async ({ page, viewer }) => {
    await page.evaluate(() => localStorage.setItem("solve-options", JSON.stringify({ objectives: [{ items: [{ name: "turns", weight: 2 }], tolerance: 0 }], beam: 8 })));
    await open(page);
    await expect(dialog(page).locator(".settings-migration")).toContainText("Old solver objective settings were removed");
    expect(await viewer.stored("solve-options")).toEqual({ beam: 8 });
    await ok(page);
    expect(await solved(page, viewer)).toEqual({ options: "beam=8", hintsForLow: false });
    await open(page);
    await expect(dialog(page).locator(".settings-migration")).toHaveCount(0);
    await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({});
  });

  test("scalar values are checked by the host without rounding", async ({ page, viewer }) => {
    await open(page);
    await field(page, "Beam").fill("0");
    await page.evaluate(() => { window.fakeShell.optionsError = "beam must be a positive count"; });
    await dialog(page).getByRole("button", { name: "OK", exact: true }).click();
    await expect(dialog(page).locator(".dialog-error")).toContainText("beam must be");
    expect(await viewer.stored("solve-options")).toBeNull();
    await page.evaluate(() => { window.fakeShell.optionsError = null; });
    await field(page, "Beam").fill("2");
    await ok(page);
    expect(await solved(page, viewer)).toEqual({ options: "beam=2", hintsForLow: false });
  });

  test("a pending objective migration notice survives native settings migration only once", async ({ page, viewer }) => {
    await page.evaluate(() => {
      localStorage.setItem("solve-options", JSON.stringify({ beam: "8" }));
      localStorage.setItem("solve-objectives-migration-notice", JSON.stringify(true));
    });
    await viewer.open("T4-miller-conflicts", { platform: "desktop", settings: "1" });
    expect(await page.evaluate(() => window.fakeShell.legacy["solve-objectives-migration-notice"])).toBe(true);
    expect(await page.evaluate(() => localStorage.getItem("solve-objectives-migration-notice"))).toBeNull();
    expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["solve-objectives-migration-notice"])).toBe(true);
    await open(page);
    await expect(dialog(page).locator(".settings-migration")).toContainText("Old solver objective settings were removed");
    await expect(field(page, "Beam")).toHaveValue("8");
    await ok(page);
    await expect.poll(async () => page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["solve-objectives-migration-notice"])).toBe(false);
    await viewer.open("T4-miller-conflicts", { platform: "desktop", settings: "1" });
    await open(page);
    await expect(dialog(page).locator(".settings-migration")).toHaveCount(0);
    await expect(field(page, "Beam")).toHaveValue("8");
    expect(await page.evaluate(() => JSON.parse(sessionStorage.getItem("fake-settings"))["solve-options"])).toEqual({ beam: "8" });
    await ok(page);
  });

  test("changing configuration reloads defaults and preserves only explicit overrides", async ({ page, viewer }) => {
    await page.evaluate(() => {
      window.fakeShell.configDefaults = {
        "A.toml": { source: "A.toml", beam: "2", tolerance: "1/8" },
        "B.toml": { source: "B.toml", beam: "8", tolerance: "3/8" },
      };
      localStorage.setItem("solve-options", JSON.stringify({ solver_config: "A.toml" }));
    });
    await open(page);
    await expect(field(page, "Beam")).toHaveValue("2");
    await field(page, "Tolerance").fill("0.10000000000000001");
    await page.evaluate(() => { window.fakeShell.configPick = "B.toml"; });
    await dialog(page).getByRole("button", { name: "Choose File…" }).click();
    await expect(field(page, "Beam")).toHaveValue("8");
    await expect(field(page, "Tolerance")).toHaveValue("0.10000000000000001");
    await expect(dialog(page).locator(".settings-source")).toContainText("B.toml");
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({ solver_config: "B.toml", tolerance: "0.10000000000000001" });
    await open(page);
    await dialog(page).getByRole("button", { name: "Restore Defaults" }).click();
    await expect(field(page, "Beam")).toHaveValue("4");
    await expect(field(page, "Tolerance")).toHaveValue("0.000001");
    await expect(dialog(page).locator(".settings-source")).toContainText("builtin");
    await ok(page);
    expect(await viewer.stored("solve-options")).toEqual({});
  });

  test("latest typed path wins out of order and saving waits for its defaults", async ({ page, viewer }) => {
    await open(page);
    await page.evaluate(() => {
      window.fakeShell.configDefaults = {
        "A.toml": { source: "A.toml", beam: "2" },
        "B.toml": { source: "B.toml", beam: "8" },
        "C.toml": { source: "C.toml", beam: "16" },
      };
      window.fakeShell.waitConfigDefaults = ["A.toml", "B.toml", "C.toml"];
    });
    await field(page, "Configuration file").fill("A.toml");
    await field(page, "Configuration file").fill("B.toml");
    await page.waitForFunction(() => window.fakeShell.pendingDefaults?.length === 2);
    await page.evaluate(() => window.fakeShell.pendingDefaults.find((p) => p.path === "B.toml").finish());
    await expect(field(page, "Beam")).toHaveValue("8");
    await page.evaluate(() => window.fakeShell.pendingDefaults.find((p) => p.path === "A.toml").finish());
    await expect(dialog(page).locator(".settings-source")).toContainText("B.toml");
    await expect(field(page, "Beam")).toHaveValue("8");
    await field(page, "Configuration file").fill("C.toml");
    await page.waitForFunction(() => window.fakeShell.pendingDefaults?.length === 3);
    await dialog(page).getByRole("button", { name: "OK", exact: true }).click();
    expect(await viewer.stored("solve-options")).toBeNull();
    await page.evaluate(() => window.fakeShell.pendingDefaults.find((p) => p.path === "C.toml").finish());
    await expect(dialog(page)).toHaveCount(0);
    expect(await viewer.stored("solve-options")).toEqual({ solver_config: "C.toml" });
    expect((await viewer.calls("checkSolverOptions")).at(-1)).toEqual(["", "C.toml"]);
  });

  test("editing back to an exact default sends a parseable decimal without rounding", async ({ page, viewer }) => {
    await page.evaluate(() => {
      window.fakeShell.configDefaults = { builtin: { tolerance: "10000000000000001/100000000000000000" } };
    });
    await open(page);
    await expect(field(page, "Tolerance")).toHaveValue("0.10000000000000001");
    await field(page, "Tolerance").fill("0.2");
    await field(page, "Tolerance").fill("0.10000000000000001");
    await ok(page);
    expect((await viewer.calls("checkSolverOptions")).at(-1)).toEqual(["tolerance=0.10000000000000001", null]);
    expect(await viewer.stored("solve-options")).toEqual({ tolerance: "0.10000000000000001" });
  });

  test("a path changed during validation is checked again before saving", async ({ page, viewer }) => {
    await open(page);
    await page.evaluate(() => { window.fakeShell.waitOptionsChecks = true; });
    await field(page, "Configuration file").fill("A.toml");
    await dialog(page).getByRole("button", { name: "OK", exact: true }).click();
    await page.waitForFunction(() => window.fakeShell.pendingChecks?.length === 1);
    await field(page, "Configuration file").fill("B.toml");
    await page.evaluate(() => window.fakeShell.pendingChecks[0].finish());
    await page.waitForFunction(() => window.fakeShell.pendingChecks?.length === 2);
    expect(await viewer.stored("solve-options")).toBeNull();
    expect((await viewer.calls("checkSolverOptions")).at(-1)).toEqual(["", "B.toml"]);
    await page.evaluate(() => window.fakeShell.pendingChecks[1].finish());
    await expect(dialog(page)).toHaveCount(0);
    expect(await viewer.stored("solve-options")).toEqual({ solver_config: "B.toml" });
  });
});
