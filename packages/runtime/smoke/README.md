# smoke

A minimal frontend built with Vite that installs `@rich-netlist/runtime` as npm installs it, and [`smoke.mjs`](smoke.mjs), which runs it:

```bash
cd packages/runtime/smoke
npm ci
node smoke.mjs ../../../rich-netlist-runtime-0.1.0-preview.9.tgz
node smoke.mjs @rich-netlist/runtime@next
```

The argument is what npm installs: a tarball that `npm pack` wrote, or a version on npm. `smoke.mjs` installs it without saving it, builds the page with Vite ([`vite.config.js`](vite.config.js) has the two settings of the package's README), checks that the build has the Worker and the module, and opens the build and then the dev server in Chromium. The page, [`main.js`](main.js), builds a netlist of 0.3 that draws a symbol of `analog-canvas-extensions` and writes an edit into a netlist; the test requires no error, a level drawn with its SVG, and the edit written.

[`publish-runtime.yml`](../../../.github/workflows/publish-runtime.yml) runs it on the tarball before it publishes, and on the version from npm after. Vite and Playwright are pinned in `package.json` and its lock file: Vite at the version Analog Canvas builds with, Playwright at the version of the viewer's tests.

Before running the smoke fixture, run `python scripts/packages.py download` from the repository root. Vite resolves `rnl-public-canvas?url` to the verified locked cache through `scripts/packages.mjs`; it no longer imports optional data from the npm package. Missing data skips locally and fails in CI.
