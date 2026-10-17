// The npm package as publish-runtime.yml publishes it, checked before anything is published: what
// `npm pack` puts into the tarball of a directory that build.mjs wrote with a name and a version.
//
//   node bindings/runtime-wasm/tests/package.mjs DIR VERSION
//
// The tarball must hold the module, the JavaScript modules with their declarations, the README and
// the licenses, and nothing else. Its package.json must name @rich-netlist/runtime at VERSION, not be
// private, be public on npm, name the repository that npm's provenance checks, give the license of
// what the package holds, and export files that are in it. The build must bring no packages of its
// own, as build.mjs --packages would: the core bundles the standard packages.

import { execFileSync, execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const [dir, version] = process.argv.slice(2);
if (!dir || !version) {
  console.error("usage: node bindings/runtime-wasm/tests/package.mjs DIR VERSION");
  process.exit(2);
}

const problems = [];
const want = (what, ok) => ok || problems.push(what);

const pkg = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
want(`the name @rich-netlist/runtime, not ${pkg.name}`, pkg.name === "@rich-netlist/runtime");
want(`the version ${version}, not ${pkg.version}`, pkg.version === version);
want("not private", !pkg.private);
want("public on npm (publishConfig.access)", pkg.publishConfig?.access === "public");
want("the license MIT AND CC0-1.0 AND OFL-1.1", pkg.license === "MIT AND CC0-1.0 AND OFL-1.1");
want("the repository github.com/rich-netlist/rnl", /^git\+https:\/\/github\.com\/rich-netlist\/rnl\.git$/.test(pkg.repository?.url ?? ""));
want("client.js as the package", pkg.exports?.["."]?.default === "./client.js");

// npm pack --dry-run --json lists the files of the tarball without writing it; on Windows npm is a
// command script, which only a shell runs
const npm = (args) =>
  process.platform === "win32"
    ? execSync(["npm", ...args.map((a) => JSON.stringify(a))].join(" "), { encoding: "utf8" })
    : execFileSync("npm", args, { encoding: "utf8" });
const [packed] = JSON.parse(npm(["pack", "--dry-run", "--json", resolve(dir)]));
const files = packed.files.map((f) => f.path.replace(/\\/g, "/")).sort();
const expected = [
  "LICENSE",
  "LICENSES/CC0-1.0.txt",
  "LICENSES/OFL-1.1.txt",
  "README.md",
  "catalog.d.ts",
  "client.d.ts",
  "client.js",
  "core.d.ts",
  "core.js",
  "document.d.ts",
  "package.json",
  "packages.js",
  "rnl_runtime_wasm.wasm",
  "snapshot.d.ts",
  "worker.js",
];
want(`the files ${expected.join(", ")}, not ${files.join(", ")}`, files.join() === expected.join());
const targets = [pkg.types, ...Object.values(pkg.exports ?? {}).flatMap((e) => (typeof e === "string" ? [e] : Object.values(e)))];
for (const t of targets) want(`${t}, which package.json names, in the tarball`, files.includes(t.replace(/^\.\//, "")));

// no packages of the build's own: the core bundles them
const packages = readFileSync(join(dir, "packages.js"), "utf8");
want("no packages in packages.js (a build without --packages)", /export const files = \{\};/.test(packages) && /export const packageDirs = \[\];/.test(packages));

if (problems.length) {
  for (const p of problems) console.error(`FAIL ${p}`);
  process.exit(1);
}
console.log(`ok   ${pkg.name}@${pkg.version}: ${files.length} files, ${packed.size} bytes packed, ${packed.unpackedSize} unpacked`);
