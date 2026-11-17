// The real desktop shell, with only the Tauri bridge mocked: what it hands `invoke` as its third argument, the options.
// write_bytes (Save Project, Open Project) reads the file's path from a header, and save_image (Export Image) its default
// name and its kind; an `invoke` that drops the options, as the shell's once did, leaves the first without a path
// ("write_bytes needs a path") and makes Export PNG offer drawing.svg.
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
    return recorded.filter((c) => ["write_bytes", "save_image", "pick_project_save", "export_svg"].includes(c.command));
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

test("Save Project gives write_bytes the archive's path as a header", async ({ page, viewer }) => {
  const recorded = await calls(page, viewer, { pick_project_save: "C:/p/a b.zip" }, async (desktop) => {
    await desktop.saveProject("a", async () => new Blob(["12345"]));
  });
  expect(recorded).toEqual([
    { command: "pick_project_save", args: { name: "a", last: null }, options: null },
    { command: "write_bytes", args: { bytes: 5 }, options: { headers: { path: "C%3A%2Fp%2Fa%20b.zip" } } },
  ]);
});

test("Open Project writes each file of the project with its path as a header", async ({ page, viewer }) => {
  const recorded = await calls(page, viewer, {}, async (desktop) => {
    const bytes = { "C:/p/1-first/a.sp": new Uint8Array([1, 2]), "C:/p/2-second/b c.sp": new Uint8Array([3]) };
    await desktop.projectSource({ bytes, entry: "C:/p/1-first/a.sp", pool: Object.keys(bytes) }, { path: "C:/p.zip" });
  });
  expect(recorded).toEqual([
    { command: "write_bytes", args: { bytes: 2 }, options: { headers: { path: "C%3A%2Fp%2F1-first%2Fa.sp" } } },
    { command: "write_bytes", args: { bytes: 1 }, options: { headers: { path: "C%3A%2Fp%2F2-second%2Fb%20c.sp" } } },
  ]);
});
