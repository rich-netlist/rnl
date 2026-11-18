// The real desktop shell, with only the Tauri bridge mocked: what it hands `invoke` as its third argument, the options.
// write_project_file (Open Project) reads the file's path within the project's folder from a header, and save_image
// (Export Image) its default name and its kind; an `invoke` that drops the options, as the shell's once did, leaves the
// first without a path and makes Export PNG offer drawing.svg. write_project_archive (Save Project) takes no path: the app
// writes where its own dialog was answered.
import { expect, test } from "../viewer.js";

/** Mock the bridge, and call the real shell with `body`, which gets it as `desktop` and returns what it likes; give the calls. */
async function calls(page, viewer, answers, body) {
  await viewer.open("04-hierarchy", { platform: "desktop" });
  return page.evaluate(async ({ answers, body }) => {
    const recorded = [];
    window.__TAURI__ = {
      event: { listen: async () => () => {} },
      core: {
        invoke: async (command, args, options) => {
          recorded.push({ command, args: args instanceof Uint8Array ? { bytes: args.length } : (args ?? null), options: options ?? null });
          return answers[command];
        },
      },
      webview: { getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) },
    };
    const desktop = (await import("../shells/desktop.js")).default;
    await new Function("desktop", `return (${body})(desktop)`)(desktop);
    return recorded.filter((c) => ["write_project_archive", "write_project_file", "save_image", "pick_project_save", "export_svg"].includes(c.command));
  }, { answers, body: body.toString() });
}

test("Export Image gives save_image the image's name and kind as headers", async ({ page, viewer }) => {
  const recorded = await calls(page, viewer, {}, async (desktop) => { await desktop.saveImage("amp 1.png", new Blob(["abc"]), "png"); });
  expect(recorded).toEqual([{ command: "save_image", args: { bytes: 3 }, options: { headers: { name: "amp%201.png", kind: "png" } } }]);
});

test("Export Image tells export_svg whether the document shown is a solve's preview", async ({ page, viewer }) => {
  const recorded = await calls(page, viewer, { export_svg: "<svg/>" }, async (desktop) => {
    const request = { level: 0, title: "amp", overlays: {}, source: { path: "C:/w/amp.sp" }, edits: null };
    await desktop.exportSvg({ ...request, preview: true });
    await desktop.exportSvg(request);
  });
  expect(recorded.map((c) => [c.command, c.args.preview, c.args.path])).toEqual([
    ["export_svg", true, "C:/w/amp.sp"],
    ["export_svg", false, "C:/w/amp.sp"],
  ]);
});

test("Save Project writes the archive with no path of its own, where the dialog was answered, and offers it next time", async ({ page, viewer }) => {
  const recorded = await calls(page, viewer, { pick_project_save: "C:/p/a b.zip" }, async (desktop) => {
    await desktop.saveProject("a", async () => new Blob(["12345"]));
    await desktop.saveProject("a", async () => new Blob(["123"]));
  });
  expect(recorded).toEqual([
    { command: "pick_project_save", args: { name: "a", last: null }, options: null },
    { command: "write_project_archive", args: { bytes: 5 }, options: null },
    { command: "pick_project_save", args: { name: "a", last: "C:/p/a b.zip" }, options: null },
    { command: "write_project_archive", args: { bytes: 3 }, options: null },
  ]);
});

test("Save Project writes nothing when its dialog is cancelled", async ({ page, viewer }) => {
  const recorded = await calls(page, viewer, { pick_project_save: null }, async (desktop) => {
    await desktop.saveProject("a", async () => new Blob(["12345"]));
  });
  expect(recorded).toEqual([{ command: "pick_project_save", args: { name: "a", last: null }, options: null }]);
});

test("Open Project writes each file of the project with its path within the chosen folder as a header", async ({ page, viewer }) => {
  const answers = { pick_project_folder: "C:\\p\\", existing_files: [] };
  const recorded = await calls(page, viewer, answers, async (desktop) => {
    const bytes = { "C:/p/1-first/a.sp": new Uint8Array([1, 2]), "C:/p/2-second/b c.sp": new Uint8Array([3]) };
    await desktop.projectRoot({ path: "C:/p.zip" }, { files: [{ path: "1-first/a.sp" }, { path: "2-second/b c.sp" }] });
    await desktop.projectSource({ bytes, entry: "C:/p/1-first/a.sp", pool: Object.keys(bytes) }, { path: "C:/p.zip" });
  });
  expect(recorded).toEqual([
    { command: "write_project_file", args: { bytes: 2 }, options: { headers: { path: "1-first%2Fa.sp" } } },
    { command: "write_project_file", args: { bytes: 1 }, options: { headers: { path: "2-second%2Fb%20c.sp" } } },
  ]);
});

test("Open Project writes no file outside the chosen folder, nor any before a folder is chosen", async ({ page, viewer }) => {
  const answers = { pick_project_folder: "C:/p", existing_files: [] };
  const recorded = await calls(page, viewer, answers, async (desktop) => {
    const outside = { bytes: { "C:/q/a.sp": new Uint8Array([1]) }, entry: "C:/q/a.sp", pool: [] };
    const failures = [];
    try { await desktop.projectSource(outside, { path: "C:/p.zip" }); } catch (e) { failures.push(e.message); }
    await desktop.projectRoot({ path: "C:/p.zip" }, { files: [{ path: "a.sp" }] });
    // a name that only begins as the folder's does is not in it
    const beside = { bytes: { "C:/pq/a.sp": new Uint8Array([1]) }, entry: "C:/pq/a.sp", pool: [] };
    try { await desktop.projectSource(beside, { path: "C:/p.zip" }); } catch (e) { failures.push(e.message); }
    if (failures.length !== 2) throw new Error(`refused ${failures.length} of 2: ${failures.join("; ")}`);
  });
  expect(recorded).toEqual([]);
});
