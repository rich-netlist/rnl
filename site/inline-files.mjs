// The complete files the pages show (site/src/assets/scenes.js, RNLScenes.files()), as JSON on stdout:
// [{ "name", "expect", "text" }], the English of each <en> group kept and its markup dropped.
// site/build.py --check runs it and reads each file with rnl check.
//
//   node site/inline-files.mjs
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const context = { window: {} };
vm.runInNewContext(readFileSync(join(root, "site/src/assets/scenes.js"), "utf8"), context);
const plain = text => text.replace(/<en(?: ctx=[\w-]+)?>([\s\S]*?)<\/en>/g, "$1");
const files = context.window.RNLScenes.files().map(file => ({
  name: file.name,
  expect: file.expect,
  text: file.lines.map(line => plain(typeof line === "string" ? line : line.t)).join("\n") + "\n",
}));
process.stdout.write(JSON.stringify(files, null, 1) + "\n");
