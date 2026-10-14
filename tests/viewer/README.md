# viewer

Browser tests of the viewers' shared page, [`apps/viewer-ui`](../../apps/viewer-ui/README.md), in Chromium, Firefox and WebKit with [Playwright Test](https://playwright.dev). The page runs as it is, with a fake shell in place of the desktop's or the web's: the shell opens Documents that `rnl view` built natively before the tests, records what the page asks of it, and does what a test tells it. The tests use the page through its UI, with the mouse and the keyboard, and read the DOM and the debug handle `window.viewer`. They wait for conditions, never for a time.

They also smoke-test the web viewer, [`apps/web`](../../apps/web/README.md): its site as `build.mjs` makes it with the samples, served by `serve.mjs`, with the web shell and the real core as WebAssembly in its Worker ([The web viewer](#the-web-viewer)).

The desktop viewer has a smoke test of its own, [`apps/desktop/tests/smoke.mjs`](../../apps/desktop/README.md#testing).

## Running

Node 20 or later, and Rust with the `wasm32-unknown-unknown` target for the web viewer's core; from this directory, once:

```bash
npm ci
```

Before the tests Playwright builds the web viewer's site in `apps/web/dist` and serves it on the port `RNL_WEB_PORT` names, or a free one; the global setup builds the rnl command with Cargo and writes the fixtures into a temporary directory. A command line that names spec files, none of them `web.spec.js`, such as `npx playwright test specs/text.spec.js`, builds and serves no site, so the page's own tests need no WebAssembly target; `RNL_WEB=0` leaves the site out in any case, and the web viewer's tests skip, and `RNL_WEB=1` builds it in any case. The chromium project drives the installed Google Chrome, headless and with a profile of its own, so no Chromium is downloaded; `RNL_CHANNEL` names another of Playwright's channels instead, such as `chromium` for Playwright's own build. Firefox and WebKit need Playwright's builds, `npx playwright install firefox webkit` once. Two workers run at a time, each with a browser of its own.

```bash
npx playwright test --project chromium
npx playwright test --project firefox
npx playwright test                      # all three
```

In CI, [`viewer.yml`](../../.github/workflows/viewer.yml) installs Rust with the WebAssembly target and Playwright's three browsers with what they need (`npx playwright install --with-deps`), and runs all three projects in them. To look at the page with the fake shell outside the tests, `node server.js [PORT]` builds the fixtures and serves them, for example at `http://127.0.0.1:PORT/?shell=fake&open=04-hierarchy`.

## Files

| Path | Content |
| --- | --- |
| `playwright.config.js` | The three projects, the browser of each, and `RNL_CHANNEL`; the web viewer's server, when the run can include its tests, which find it at `RNL_WEB_URL`. |
| `global-setup.js` | Builds the rnl command, writes the fixtures and starts the server; the tests find it at `RNL_VIEWER_URL`, and the rnl command at `RNL_BIN`. |
| `fixtures.js` | The fixtures: `rnl view` of the corpus files `01-rc-lowpass.sp`, `04-hierarchy.sp`, `10-constraints-and-priority.sp`, `12-diagnostics.sp`, `17-line-styles.sp` and `inverter-chain.scs`; `04-hierarchy-changed`, the same netlist after it changed; `subcircuits`, a netlist with a subcircuit for each initial; `nested`, subcircuits nested four deep, and `nested-changed`, the same netlist after an instance on the way down was renamed; and `continued` and `continued-spectre`, statements over several lines in each dialect. |
| `server.js` | The test server, Node with no dependencies: `apps/viewer-ui` as it is, the fake shell as `shells/fake.js`, and the fixtures under `fixtures/`. Nothing of the tests goes into the page. |
| `fake-shell.js` | The fake shell. |
| `viewer.js` | What the tests share: opening the page, waiting for a build, the page's state, and where an object of the drawing is on the screen. |
| `specs/` | The tests, one file per part of the page: opening, selection, subcircuits, the hierarchy path, the hierarchy view, the text pane, the view, the grid, the keys, reading again, the menu bar and its submenus, and the panels; and `web.spec.js`, the web viewer's smoke tests. The grid's tests read the pixels of its canvas where the scene's points are drawn. |

## The fake shell

It implements the Shell of [`shell.js`](../../apps/viewer-ui/shell.js). A source is a fixture's name, or `{ name, fixture }`. The page's URL sets it up:

| Parameter | |
| --- | --- |
| `open=NAME` | Open the fixture `NAME` at start. |
| `fail-start=MESSAGE` | Reject the question of what to open at start with the plain message, as Tauri's invoke does. |
| `platform=desktop` | Offer the desktop's commands with a native menu, so that the page shows no menu bar; otherwise the web's. |

`window.fakeShell` holds what the page asked and what the shell does next:

| Member | |
| --- | --- |
| `calls` | The calls in order: `["initial"]`, `["pick", kind]`, `["build", name]`, `["setCheck", id, checked]`, `["browse", url]`. |
| `checks` | The check marks as a native menu would show them. |
| `picks` | What the next picks choose, in order; nothing once it is empty. |
| `fail` | A message the next build rejects with. |
| `next` | A fixture the next build returns instead of its source's, as if the file had changed. |
| `menu(id)` | Choose a command in the platform's menu. |

## The web viewer

`specs/web.spec.js` uses the site as a browser does, with no fake shell and nothing of the tests in the page. It opens by `?url=` the samples `cases/04-hierarchy.sp`, whose include it fetches, `cases/01-rc-lowpass.sp`, `cases/12-diagnostics.sp` and `equivalence/inverter-chain.scs`; files set on `#file-input`, a netlist with its include and two netlists, for which the page asks which to open; a folder set on `#folder-input`; files dropped on the page, in a `drop` event the test makes; a Latin-1 file; a netlist whose include is not there, and one whose include is above the files opened; a served netlist that changes, with its include, before Reload; and from a web server of the test's own, which lets the page read it (CORS), addresses that redirect to a netlist, which open as its own address does, its include above it too, and a netlist at the server's root, named after the server. Each Document must be the one `rnl view` builds for the same files, deep-equal but for the paths that name them in the workspace: the entry's, and those of the files missing. The tests also check what the page shows of it: the drawing, the file tabs and the lines, the problems and the hierarchy; that Help > About names the version and the commit of `about.json`; and that Help > Desktop Viewer opens the project site's page of the tools in a new tab, which the test answers itself rather than fetch the site. A test whose served file changes writes it into a directory of its own under `apps/web/dist`, and removes it after.

## Known failures

No test is marked as failing. The three that were, an initial of the subcircuit index that scrolled to the end of the list, a double click on a tree row that did not fold the tree in Firefox, and the file tabs that the failed statements' box covered at the narrowest side panel, pass since the page was fixed.

The test of dropped files is skipped in WebKit: a drop made in the page gives its items entries whose files WebKit cannot read (`NotFoundError`). Chromium gives such a drop no entries, so the page reads its files as with Open, and Firefox gives entries that read them.
