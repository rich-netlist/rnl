# viewer

Browser tests of the viewers' shared page, [`apps/viewer-ui`](../../apps/viewer-ui/README.md), in Chromium, Firefox and WebKit (CI runs Chromium and WebKit) with [Playwright Test](https://playwright.dev). The page runs as it is, with a fake shell in place of the desktop's or the web's: the shell opens Documents that `rnl view` built natively before the tests, records what the page asks of it, and does what a test tells it. The tests use the page through its UI, with the mouse and the keyboard, and read the DOM and the debug handle `window.viewer`. They wait for conditions, never for a time. A dialog that a test closes is waited for with `dialogGone(page)` (`viewer.js`) before the next command: the dialog stops being `open`, and a check by role stops seeing it, before the page counts it closed and takes commands again, so a command sent in between is dropped.

They also smoke-test the web viewer, [`apps/web`](../../apps/web/README.md): its site as `build.mjs` makes it with the samples, served by `serve.mjs`, with the web shell and the real core as WebAssembly in its Worker ([The web viewer](#the-web-viewer)).

The desktop viewer has a smoke test of its own, [`apps/desktop/tests/smoke.mjs`](../../apps/desktop/README.md#testing).

## Running

Node 20 or later, and Rust with the `wasm32-unknown-unknown` target for the web viewer's core; from this directory, once:

```bash
npm ci
```

Before the tests Playwright builds the web viewer's site in `apps/web/dist` and serves it on the port `RNL_WEB_PORT` names, or a free one; the global setup builds the rnl command with Cargo and writes the fixtures into a temporary directory. A command line that names spec files, none of them `web.spec.js`, such as `npx playwright test specs/text.spec.js`, builds and serves no site, so the page's own tests need no WebAssembly target; `RNL_WEB=0` leaves the site out in any case, and the web viewer's tests skip, and `RNL_WEB=1` builds it in any case. The chromium project drives the installed Google Chrome, headless and with a profile of its own, so no Chromium is downloaded; `RNL_CHANNEL` names another of Playwright's channels instead, such as `chromium` for Playwright's own build. Firefox and WebKit need Playwright's builds, `npx playwright install firefox webkit` once. Two workers run at a time, each with a browser of its own.

The test of a file of 200 000 lines, `long.spec.js`, runs in a project of its own, `long`, in Chromium only: it guards against Chrome's limit on the arguments of a call, and in Firefox and WebKit it would only add load. It depends on the three other projects, so it runs after them and alone, and does not slow down their tests on a busy machine; `--no-deps` runs it without them.

The tests of the page on a phone or a tablet, `mobile*.spec.js`, run in two projects of their own, `mobile-chromium` and `mobile-webkit`, and in no other ([Mobile](#mobile)).

```bash
npx playwright test --project chromium
npx playwright test --project firefox
npx playwright test --project mobile-chromium --project mobile-webkit
npx playwright test --project long --no-deps   # the long file alone
npx playwright test                            # all six, long last
```

In CI, [`viewer.yml`](../../.github/workflows/viewer.yml) installs Rust with the WebAssembly target and Playwright's Chromium and WebKit with what they need (`npx playwright install --with-deps chromium webkit`), and runs the chromium, webkit, mobile-chromium and mobile-webkit projects, two workers at a time; the firefox project runs only locally. To look at the page with the fake shell outside the tests, `node server.js [PORT]` builds the fixtures and serves them, for example at `http://127.0.0.1:PORT/?shell=fake&open=04-hierarchy`.

## Files

| Path | Content |
| --- | --- |
| `playwright.config.js` | The three projects of the browsers, `mobile-chromium` and `mobile-webkit`, a phone in Chromium and in WebKit, and `long`, the long file's, in Chromium after the browsers'; the browser of each, and `RNL_CHANNEL`; the web viewer's server, when the run can include its tests, which find it at `RNL_WEB_URL`. |
| `global-setup.js` | Builds the rnl command, writes the fixtures and starts the server; the tests find it at `RNL_VIEWER_URL`, and the rnl command at `RNL_BIN`. |
| `fixtures.js` | The fixtures: `rnl view` of the corpus files `01-rc-lowpass.sp`, `04-hierarchy.sp`, `10-constraints-and-priority.sp`, `12-diagnostics.sp`, `17-line-styles.sp` and `inverter-chain.scs`; `04-hierarchy-changed`, the same netlist after it changed; `subcircuits`, a netlist with a subcircuit for each initial; `nested`, subcircuits nested four deep, and `nested-changed`, the same netlist after an instance on the way down was renamed; `continued` and `continued-spectre`, statements over several lines in each dialect; `unicode`, an emoji and a CJK character of the extension planes, which JavaScript counts as two UTF-16 units and the Document as one character, before references of records, in a name and before a statement that fails; `long`, a file of 200 004 lines, more than one call takes as its arguments in Chrome; `T2-common-source-solved` and `T3-ota5t-solved`, `rnl view` of the solver's test netlists `T2-common-source.sp` and `T3-ota5t.sp` of `tests/integration/solver` after `rnl solve` solved them; `edit`, a small netlist, `edit-changed`, the same file after an edit that renames C1, and `edit-crlf`, the first with CRLF line ends, for editing the text; and `T4-miller-conflicts`, `rnl view` of that netlist, with `T4-miller-conflicts-preview`, what `rnl preview` prints of it, the solve that the desktop's Solve menu shows, with facts discarded and hints by statement; `unplaced`, a netlist of 60 resistors, a capacitor, a transistor and a subcircuit instance without a position, more than the list of what is not drawn adds at once; and the netlists with a top level that has no objects, for the subcircuits that nothing calls: `uncalled`, a cell and its testbench, and `uncalled-spectre`, the same in Spectre; `uncalled-lib`, a library with a subcircuit that is not placed, one that calls another, and two that call each other; `uncalled-unplaced`, two subcircuits with no position; `uncalled-top`, a top level with an object that has no position, and a subcircuit nothing calls; and `nothing`, a title alone; and `pick`, two resistors whose boxes overlap, with R1's name label inside R2's box, a text attached to a terminal, and a wire through a capacitor's box, for what a click and the pointer find. |
| `server.js` | The test server, Node with no dependencies: `apps/viewer-ui` as it is, the fake shell as `shells/fake.js`, and the fixtures under `fixtures/`. Nothing of the tests goes into the page. |
| `fake-shell.js` | The fake shell. |
| `viewer.js` | What the tests share: opening the page, waiting for a build, the page's state, and where an object of the drawing is on the screen. |
| `mobile.js` | What the mobile tests share: `fingers`, one or two on a touch screen, the view of the drawing, and whether the page itself scrolled or zoomed. |
| `specs/` | The tests, one file per part of the page: opening, selection, subcircuits, the hierarchy path, the hierarchy view, the text pane, the view, the grid, the keys, reading again, the menu bar and its submenus, the panels, checking for updates, with GitHub's answer faked by `page.route`, loading from the clipboard, whose text the test gives the page, the text dimmed outside the subcircuit drawn, a file of 200 004 lines, which `long.spec.js` opens in the project `long` and without Playwright's trace, as the trace's snapshot of so many lines at each step takes seconds, the subcircuits that nothing calls, listed in the tree below the top level, with the level a document opens on when the top level has no objects and the messages of a level with nothing to draw, `uncalled.spec.js`, the objects that are not drawn, listed in a view of the side panel and greyed in the netlist, with their drawings asked for as the list adds its rows (one request for a part, the places in the level's list, nothing again for a row it has, a failure told on the thumbnail, drawings that come late to the row they were asked for), `unplaced.spec.js`, the name beside the pointer on an instance or a net, and View > Show Names on Hover, `hover.spec.js`, the labels on the drawing, clicked and marked with the object they are attached to, `labels.spec.js`, the list that a click on several objects opens, with the mouse and the keys, `pick.spec.js` (each on both platforms), editing the text, applying it and saving it, with the questions before changes not saved are left, `edit.spec.js`, the desktop's Solve menu, `solve.spec.js`, and its settings dialog, `solve-settings.spec.js`, with the fake shell's solve, which gives the fixture `NAME-preview` and can wait, so that the test shows progress and cancels, and the netlists the solver drew, of which `solved.spec.js` saves screenshots as `test-results/solved-T2.png`, `solved-T3-top.png` and `solved-T3.png`; and `web.spec.js`, the web viewer's smoke tests. The grid's tests read the pixels of its canvas where the scene's points are drawn. |

`virtual-selection.spec.js` checks the independent selection of ground, supply, port and junction
usages, exact box geometry at two zooms on both backgrounds and platforms, net pin circles without
virtual frames, label clicks, hover names and instance descent. `RNL_REVIEW_DIR` saves Chromium's
ground, port and wire screenshots there, on each platform and background.

## The fake shell

It implements the Shell of [`shell.js`](../../apps/viewer-ui/shell.js). A source is a fixture's name, or `{ name, fixture }`. The page's URL sets it up:

| Parameter | |
| --- | --- |
| `open=NAME` | Open the fixture `NAME` at start. |
| `fail-start=MESSAGE` | Reject the question of what to open at start with the plain message, as Tauri's invoke does. |
| `platform=desktop` | Offer the desktop's commands with a native menu, so that the page shows no menu bar, and what Help > Check for Updates needs (`updates`); otherwise the web's. |
| `version=VERSION` | The version of the app that runs, for Help > Check for Updates; 0.1.0 by default. |
| `pseudo` | Offer, besides the languages of `i18n/`, a pseudo language, `qps-ploc`, whose catalog writes every text of the others with accented letters, 40% longer, in brackets ［…］: a text of the interface that no catalog has shows as plain English among them (decision 0015). |

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
| `buildFormats`, `detected`, `keepsFormat` | The format choices each build was given, `{ path: format }`, in order, which the shell's Document then reads its files in; the defaults `detectFormat` gives by path, else by the `.scs` extension; and whether a choice is kept in the settings, true by default. A fixture without the core's format fields gets them: the two formats, and each host file's default by its extension. |
| `changedOnDisk`, `overwrites` | The names of files that another program changed since they were read, which the next Save, unless it overwrites, reports instead of writing; and whether each Save was to overwrite. |
| `writes`, `writesAs`, `writeFail`, `savedAs` | The files each Save and each Save As (desktop) was asked to write, a message the next one rejects with, and what the next Save As gives: a source, or null when the user cancels. |
| `requestClose()` | Ask the page to close the window, as the desktop does while changes are not saved. |
| `carry`, `drawingsFail`, `holdDrawings`, `releaseDrawings()` | A build gives its Document as the real cores do, without the drawings of the objects that have no position, and keeps them for `unplacedDrawings(level, items, names)`, which records `["unplacedDrawings", level, items]` and refuses names that are not the last build's. `carry` makes the next builds carry them in the Document, as a solve's does; `drawingsFail` is a message the next request rejects with; `holdDrawings` makes requests wait until `releaseDrawings()`. |

## The web viewer

`specs/embed.spec.js` exercises the canvas-only web viewer in real iframes with the WebAssembly
core, using Chromium and WebKit and at most two workers. It covers URL settings and storage
isolation, toolbar and hierarchy navigation, postMessage opens and replies, origin checks,
errors and overlapping requests, focus, resizing, language labels and all four demo examples.
Run `npx playwright test specs/embed.spec.js --project chromium --project webkit --workers 2`.
This command builds the web site but skips native CLI/fake-shell fixture setup. With a core
already built, `RNL_WEB_NO_CORE=1` reuses it. `RNL_EMBED_REVIEW_DIR` saves light/dark iframe and
before/after demo screenshots in that directory (Chromium only).

`specs/web.spec.js` uses the site as a browser does, with no fake shell and nothing of the tests in the page. It opens by `?url=` the samples `cases/04-hierarchy.sp`, whose include it fetches, `cases/01-rc-lowpass.sp`, `cases/12-diagnostics.sp` and `equivalence/inverter-chain.scs`; files set on `#file-input`, a netlist with its include and two netlists, for which the page asks which to open; a folder set on `#folder-input`; files dropped on the page, in a `drop` event the test makes; a Latin-1 file; a netlist whose include is not there, and one whose include is above the files opened; a served netlist that changes, with its include, before Reload; and from a web server of the test's own, which lets the page read it (CORS), addresses that redirect to a netlist, which open as its own address does, its include above it too, and a netlist at the server's root, named after the server. Each Document must be the one `rnl view` builds for the same files, deep-equal but for the paths that name them in the workspace: the entry's, and those of the files missing. The tests also check what the page shows of it: the drawing, the file tabs and the lines, the problems and the hierarchy; that Help > About names the version and the commit of `about.json`; that File > Load from Clipboard opens the clipboard's text alone, as the Document `rnl view` builds for that text in a file of its own; and that Help > Desktop Viewer opens the project site's page of the tools in a new tab, which the test answers itself rather than fetch the site. A test whose served file changes writes it into a directory of its own under `apps/web/dist`, and removes it after.

`plain-text.spec.js` opens a Verilog-A file and a Verilog file as the document (`inputs/resistor.va`, `inputs/inverter.v`): the text pane shows them with no highlighting, object places, not-drawn greying or folds, while the core's problems are still listed; a netlist is highlighted as before, and a file with a parse format chosen for it is a netlist again.

`virtual-text.spec.js` exercises offscreen diagnostics/selection/scope, multiline highlighting,
per-file and edit-mode scroll positions, full-file copy, drag selection/auto-scroll and model-based
find. `loading.spec.js` holds read/parse stages to verify visible progress without a percentage and
cleanup after success or failure. `long.spec.js` checks the 200,000-line fixture with fewer than 300
rendered rows, direct end/top scrolls and drawing-to-source navigation. Its timing excludes trace
snapshots and is a regression duration, not a native/WASM opening benchmark.

`startup.spec.js` blocks stylesheets and modules to check that the inline startup screen already
paints in light/dark mode, then resumes normal startup; a failed required stylesheet reports an
error instead of leaving the startup screen stuck.

## Mobile

The page on a phone and a tablet, the web viewer's: touch on the drawing, the layout of a small screen, and the desktop's mouse kept. `mobile-chromium` is a Pixel 7 (412×839, touch, a mobile viewport) in Chromium, and `mobile-webkit` an iPhone 15 (393×659) in WebKit; a group of tests may hold the phone lying down (734×343), be an iPad Mini (768×1024, either way up), a laptop with a touch screen (1200×800, touch and a mouse) or a narrow desktop window (420×800, a mouse alone).

| File | |
| --- | --- |
| `specs/mobile-gestures.spec.js` | One finger drags the view with it and selects nothing; two pinch it around their middle and drag it as they move; lifting one of two leaves the other dragging, with no jump; a tap selects what a click selects, also when the finger moves a few pixels, and on nothing clears; two fingers that tap select nothing; a double tap fits the drawing, or opens a subcircuit instance, once; a long press opens no menu and selects no text; none of it scrolls or zooms the page. On a laptop with a touch screen the mouse drags, zooms and clicks as on a desktop, and fingers work beside it. |
| `specs/mobile-layout.spec.js` | On a phone, upright or lying down, the drawing and the side panel are two pages, each the whole screen above a low switch bar, `#page-switch` with `#page-drawing` and `#page-panel`, which lying down is at the end of the status bar's row: switching keeps the view, the selection, the panel's view and where the netlist is scrolled; Show in Netlist (`#show-in-netlist`) and Show in Drawing (`#show-in-drawing`) go from one page to the other with the selection; opening a level or a file goes to the drawing's page. On a tablet upright the side panel is below the drawing, as wide as the screen, folds to its tabs, a tap on a tab unfolds it, and its handle, `#vsplit`, drags it taller or lower within limits, kept for the next start; lying down it is beside the drawing. Everywhere the page never scrolls sideways, whatever view of the side panel is shown; the netlist shows at least five lines of the file beside the problems, which take the height of what they list (lying down they start folded); the menu titles, the tabs and the buttons are 44 pixels each way, the menus' items 40 high; every menu and submenu opens with a tap and fits on the screen, a tap elsewhere only closes it, neither selecting nor clearing, and a tap on an item runs it; the menu bar, the level bar and the status bar fit in the screen's width, also with an object selected, which the status bar names. In a narrow desktop window the mouse drags and zooms the drawing and switches the pages. |
| `specs/mobile-symbol.spec.js` | The same fingers on the symbol preview of the Symbol view, by the same code: two spread and pinch it around their middle and drag it as they move, one drags it, a double tap fits it, the buttons on it are tapped as before, and none of it scrolls or zooms the page. |
| `specs/mobile-web.spec.js` | The web viewer's site on a phone: a sample opens by its address, fits on the screen, and a finger drags and taps the drawing. It needs the site, as `web.spec.js` does. |

Chromium gets its fingers through the DevTools protocol's `Input.dispatchTouchEvent`, so the browser makes the pointer events, honours `touch-action` and recognises taps and double taps as a phone's does, and the tests can tell whether the page itself scrolled or zoomed (`visualViewport.scale`). WebKit has no way in Playwright to put down more than one finger: there the tests send the page pointer events of their own, `pointerType` "touch", to the element under each finger at its press, as a browser captures a touch pointer, and let the page capture them (`mobile.js`); they test what the page does with two fingers, not how WebKit makes them, and a gesture's effect on the page itself is tested in Chromium only. Taps are Playwright's `touchscreen.tap` in both. What only a real phone shows, such as Safari's own gestures, the on-screen keyboard and the notch, is checked by hand, on screenshots and on devices.

## Known failures

No test is marked as failing. The three that were, an initial of the subcircuit index that scrolled to the end of the list, a double click on a tree row that did not fold the tree in Firefox, and the file tabs that the failed statements' box covered at the narrowest side panel, pass since the page was fixed.

The test of dropped files is skipped in WebKit: a drop made in the page gives its items entries whose files WebKit cannot read (`NotFoundError`). Chromium gives such a drop no entries, so the page reads its files as with Open, and Firefox gives entries that read them.
