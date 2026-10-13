# viewer

Browser tests of the viewers' shared page, [`apps/viewer-ui`](../../apps/viewer-ui/README.md), in Chromium, Firefox and WebKit with [Playwright Test](https://playwright.dev). The page runs as it is, with a fake shell in place of the desktop's or the web's: the shell opens Documents that `rnl view` built natively before the tests, records what the page asks of it, and does what a test tells it. The tests use the page through its UI, with the mouse and the keyboard, and read the DOM and the debug handle `window.viewer`. They wait for conditions, never for a time.

The desktop viewer has a smoke test of its own, [`apps/desktop/tests/smoke.mjs`](../../apps/desktop/README.md#testing).

## Running

Node 20 or later and Rust; from this directory, once:

```bash
npm ci
```

The global setup builds the rnl command with Cargo and writes the fixtures into a temporary directory. The chromium project drives the installed Google Chrome, headless and with a profile of its own, so no Chromium is downloaded; `RNL_CHANNEL` names another of Playwright's channels instead, such as `chromium` for Playwright's own build. Firefox and WebKit need Playwright's builds, `npx playwright install firefox webkit` once. Two workers run at a time, each with a browser of its own.

```bash
npx playwright test --project chromium
npx playwright test --project firefox
npx playwright test                      # all three
```

In CI, [`viewer.yml`](../../.github/workflows/viewer.yml) installs Playwright's three browsers with what they need (`npx playwright install --with-deps`) and runs all three projects in them. To look at the page with the fake shell outside the tests, `node server.js [PORT]` builds the fixtures and serves them, for example at `http://127.0.0.1:PORT/?shell=fake&open=04-hierarchy`.

## Files

| Path | Content |
| --- | --- |
| `playwright.config.js` | The three projects, the browser of each, and `RNL_CHANNEL`. |
| `global-setup.js` | Builds the rnl command, writes the fixtures and starts the server; the tests find it at `RNL_VIEWER_URL`. |
| `fixtures.js` | The fixtures: `rnl view` of the corpus files `01-rc-lowpass.sp`, `04-hierarchy.sp`, `10-constraints-and-priority.sp`, `12-diagnostics.sp` and `inverter-chain.scs`; `04-hierarchy-changed`, the same netlist after it changed; `subcircuits`, a netlist with a subcircuit for each initial; and `continued` and `continued-spectre`, statements over several lines in each dialect. |
| `server.js` | The test server, Node with no dependencies: `apps/viewer-ui` as it is, the fake shell as `shells/fake.js`, and the fixtures under `fixtures/`. Nothing of the tests goes into the page. |
| `fake-shell.js` | The fake shell. |
| `viewer.js` | What the tests share: opening the page, waiting for a build, the page's state, and where an object of the drawing is on the screen. |
| `specs/` | The tests, one file per part of the page: opening, selection, subcircuits, the hierarchy view, the text pane, the view, the keys, reading again, the menu bar, and the panels. |

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
| `calls` | The calls in order: `["initial"]`, `["pick", kind]`, `["build", name]`, `["setCheck", id, checked]`. |
| `checks` | The check marks as a native menu would show them. |
| `picks` | What the next picks choose, in order; nothing once it is empty. |
| `fail` | A message the next build rejects with. |
| `next` | A fixture the next build returns instead of its source's, as if the file had changed. |
| `menu(id)` | Choose a command in the platform's menu. |

## Known failures

Three tests are marked as failing until the page is fixed; when it is, Playwright reports that they passed unexpectedly, and `test.fail` goes then.

- An initial of the subcircuit index scrolls the list by the heading's `offsetTop`, which counts from the page's body, so every initial scrolls to the end of the list. Taking the list's `offsetTop` off is not enough: the headings are sticky, and a heading stuck at the top of the scrolled list gives the `offsetTop` of where it sticks, so going back up to an initial does not scroll. The group's first row gives where the group starts.
- At its narrowest, 220 pixels, the side panel's failed statements box, which does not shrink, takes the whole file bar, so the file tabs get no width and the box covers them: the other files cannot be shown.
- In Firefox, a double click on a row of the tree does not fold it: the row's click draws the tree again, so the row that the second click pressed is out of the page when the `dblclick` comes, and Firefox sends none. Chromium and WebKit send it to that row, whose listener still folds the tree.
