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

The test of a file of 200 000 lines, `long.spec.js`, runs in a project of its own, `long`, in Chromium only: it guards against Chrome's limit on the arguments of a call, and in Firefox and WebKit it would only add load. It depends on the three other projects, so it runs after them and alone, and does not slow down their tests on a busy machine; `--no-deps` runs it without them.

```bash
npx playwright test --project chromium
npx playwright test --project firefox
npx playwright test --project long --no-deps   # the long file alone
npx playwright test                            # all four, long last
```

In CI, [`viewer.yml`](../../.github/workflows/viewer.yml) installs Rust with the WebAssembly target and Playwright's three browsers with what they need (`npx playwright install --with-deps`), and runs all three projects in them. To look at the page with the fake shell outside the tests, `node server.js [PORT]` builds the fixtures and serves them, for example at `http://127.0.0.1:PORT/?shell=fake&open=04-hierarchy`.

## Files

| Path | Content |
| --- | --- |
| `playwright.config.js` | The three projects of the browsers, and `long`, the long file's, in Chromium after them; the browser of each, and `RNL_CHANNEL`; the web viewer's server, when the run can include its tests, which find it at `RNL_WEB_URL`. |
| `global-setup.js` | Builds the rnl command, writes the fixtures and starts the server; the tests find it at `RNL_VIEWER_URL`, and the rnl command at `RNL_BIN`. |
| `fixtures.js` | The fixtures: `rnl view` of the corpus files `01-rc-lowpass.sp`, `04-hierarchy.sp`, `10-constraints-and-priority.sp`, `12-diagnostics.sp`, `17-line-styles.sp` and `inverter-chain.scs`; `04-hierarchy-changed`, the same netlist after it changed; `subcircuits`, a netlist with a subcircuit for each initial; `nested`, subcircuits nested four deep, and `nested-changed`, the same netlist after an instance on the way down was renamed; `continued` and `continued-spectre`, statements over several lines in each dialect; `unicode`, an emoji and a CJK character of the extension planes, which JavaScript counts as two UTF-16 units and the Document as one character, before references of records, in a name and before a statement that fails; `long`, a file of 200 004 lines, more than one call takes as its arguments in Chrome; `T2-common-source-solved` and `T3-ota5t-solved`, `rnl view` of the solver's test netlists `T2-common-source.sp` and `T3-ota5t.sp` of `tests/integration/solver` after `rnl solve` solved them; `edit`, a small netlist, `edit-changed`, the same file after an edit that renames C1, and `edit-crlf`, the first with CRLF line ends, for editing the text; and `T4-miller-conflicts`, `rnl view` of that netlist, with `T4-miller-conflicts-preview`, what `rnl preview` prints of it, the solve that the desktop's Solve menu shows, with facts discarded and hints by statement; `unplaced`, a netlist of 60 resistors, a capacitor, a transistor and a subcircuit instance without a position, more than the list of what is not drawn adds at once; and the netlists with a top level that has no objects, for the subcircuits that nothing calls: `uncalled`, a cell and its testbench, and `uncalled-spectre`, the same in Spectre; `uncalled-lib`, a library with a subcircuit that is not placed, one that calls another, and two that call each other; `uncalled-unplaced`, two subcircuits with no position; `uncalled-top`, a top level with an object that has no position, and a subcircuit nothing calls; and `nothing`, a title alone; and `pick`, two resistors whose boxes overlap, with R1's name label inside R2's box, a text attached to a terminal, and a wire through a capacitor's box, for what a click and the pointer find. |
| `server.js` | The test server, Node with no dependencies: `apps/viewer-ui` as it is, the fake shell as `shells/fake.js`, and the fixtures under `fixtures/`. Nothing of the tests goes into the page. |
| `fake-shell.js` | The fake shell. |
| `viewer.js` | What the tests share: opening the page, waiting for a build, the page's state, and where an object of the drawing is on the screen. |
| `specs/` | The tests, one file per part of the page: opening, selection, subcircuits, the hierarchy path, the hierarchy view, the text pane, the view, the grid, the keys, reading again, the menu bar and its submenus, the panels, checking for updates, with GitHub's answer faked by `page.route`, loading from the clipboard, whose text the test gives the page, the text dimmed outside the subcircuit drawn, a file of 200 004 lines, which `long.spec.js` opens in the project `long` and without Playwright's trace, as the trace's snapshot of so many lines at each step takes seconds, the subcircuits that nothing calls, listed in the tree below the top level, with the level a document opens on when the top level has no objects and the messages of a level with nothing to draw, `uncalled.spec.js`, the objects that are not drawn, listed in a view of the side panel and greyed in the netlist, with their drawings asked for as the list adds its rows (one request for a part, the places in the level's list, nothing again for a row it has, a failure told on the thumbnail, drawings that come late to the row they were asked for), `unplaced.spec.js`, the name beside the pointer on an instance or a net, and View > Show Names on Hover, `hover.spec.js`, the labels on the drawing, clicked and marked with the object they are attached to, `labels.spec.js`, the list that a click on several objects opens, with the mouse and the keys, `pick.spec.js` (each on both platforms), editing the text, applying it and saving it, with the questions before changes not saved are left, `edit.spec.js`, the desktop's Solve menu, `solve.spec.js`, and its settings dialog, `solve-settings.spec.js`, with the fake shell's solve, which gives the fixture `NAME-preview` and can wait, so that the test shows progress and cancels, and the netlists the solver drew, of which `solved.spec.js` saves screenshots as `test-results/solved-T2.png`, `solved-T3-top.png` and `solved-T3.png`; and `web.spec.js`, the web viewer's smoke tests. The grid's tests read the pixels of its canvas where the scene's points are drawn. |

## The fake shell

It implements the Shell of [`shell.js`](../../apps/viewer-ui/shell.js). A source is a fixture's name, or `{ name, fixture }`. The page's URL sets it up:

| Parameter | |
| --- | --- |
| `open=NAME` | Open the fixture `NAME` at start. |
| `fail-start=MESSAGE` | Reject the question of what to open at start with the plain message, as Tauri's invoke does. |
| `platform=desktop` | Offer the desktop's commands with a native menu, so that the page shows no menu bar, and what Help > Check for Updates needs (`updates`); otherwise the web's. |
| `version=VERSION` | The version of the app that runs, for Help > Check for Updates; 0.1.0 by default. |

`window.fakeShell` holds what the page asked and what the shell does next:

| Member | |
| --- | --- |
| `calls` | The calls in order: `["initial"]`, `["pick", kind]`, `["build", name]`, `["setCheck", id, checked]`, `["browse", url]`, and on the desktop `["version"]`. |
| `checks` | The check marks as a native menu would show them. |
| `picks` | What the next picks choose, in order; nothing once it is empty. |
| `fail` | A message the next build rejects with. |
| `next` | A fixture the next build returns instead of its source's, as if the file had changed. |
| `menu(id)` | Choose a command in the platform's menu. |
| `optionsError` | On the desktop: what the solver says is wrong with the settings it is asked to check (`checkSolverOptions`), null when they will do. |
| `buildEdits`, `solveEdits` | The edits each build and each solve was given, in order; undefined for a build of the files as they are. |
| `changedOnDisk`, `overwrites` | The names of files that another program changed since they were read, which the next Save, unless it overwrites, reports instead of writing; and whether each Save was to overwrite. |
| `writes`, `writesAs`, `writeFail`, `savedAs` | The files each Save and each Save As (desktop) was asked to write, a message the next one rejects with, and what the next Save As gives: a source, or null when the user cancels. |
| `requestClose()` | Ask the page to close the window, as the desktop does while changes are not saved. |
| `carry`, `drawingsFail`, `holdDrawings`, `releaseDrawings()` | A build gives its Document as the real cores do, without the drawings of the objects that have no position, and keeps them for `unplacedDrawings(level, items, names)`, which records `["unplacedDrawings", level, items]` and refuses names that are not the last build's. `carry` makes the next builds carry them in the Document, as a solve's does; `drawingsFail` is a message the next request rejects with; `holdDrawings` makes requests wait until `releaseDrawings()`. |

## The web viewer

`specs/web.spec.js` uses the site as a browser does, with no fake shell and nothing of the tests in the page. It opens by `?url=` the samples `cases/04-hierarchy.sp`, whose include it fetches, `cases/01-rc-lowpass.sp`, `cases/12-diagnostics.sp` and `equivalence/inverter-chain.scs`; files set on `#file-input`, a netlist with its include and two netlists, for which the page asks which to open; a folder set on `#folder-input`; files dropped on the page, in a `drop` event the test makes; a Latin-1 file; a netlist whose include is not there, and one whose include is above the files opened; a served netlist that changes, with its include, before Reload; and from a web server of the test's own, which lets the page read it (CORS), addresses that redirect to a netlist, which open as its own address does, its include above it too, and a netlist at the server's root, named after the server. Each Document must be the one `rnl view` builds for the same files, deep-equal but for the paths that name them in the workspace: the entry's, and those of the files missing. The tests also check what the page shows of it: the drawing, the file tabs and the lines, the problems and the hierarchy; that Help > About names the version and the commit of `about.json`; that File > Load from Clipboard opens the clipboard's text alone, as the Document `rnl view` builds for that text in a file of its own; and that Help > Desktop Viewer opens the project site's page of the tools in a new tab, which the test answers itself rather than fetch the site. A test whose served file changes writes it into a directory of its own under `apps/web/dist`, and removes it after.

## Known failures

No test is marked as failing. The three that were, an initial of the subcircuit index that scrolled to the end of the list, a double click on a tree row that did not fold the tree in Firefox, and the file tabs that the failed statements' box covered at the narrowest side panel, pass since the page was fixed.

The test of dropped files is skipped in WebKit: a drop made in the page gives its items entries whose files WebKit cannot read (`NotFoundError`). Chromium gives such a drop no entries, so the page reads its files as with Open, and Firefox gives entries that read them.
