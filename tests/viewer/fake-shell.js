// The fake shell: the Shell of apps/viewer-ui/shell.js for the page tests, which the test server
// serves as shells/fake.js, so that `?shell=fake` loads it. It opens the fixtures, Documents that
// the global setup built natively with `rnl view`; it records what the page asks of it, and does
// what a test tells it through `window.fakeShell`.
//
// The URL sets it up: `open=NAME` opens the fixture NAME at start; `fail-start=MESSAGE` rejects the
// question of what to open at start with that plain message, as Tauri's invoke rejects; and
// `platform=desktop` offers the desktop's commands with the menu shown natively, as the desktop
// shell does; otherwise it offers the web's commands and the page shows its own menu bar.

const params = new URLSearchParams(location.search);
const platform = params.get("platform") === "desktop" ? "desktop" : "web";

/** A source: a fixture, by its name or as { name, fixture }; null for nothing. */
const source = (s) => (s === null || s === undefined ? null : typeof s === "string" ? { name: s, fixture: s } : s);

let handler = null; // the page's handler of the commands chosen in the platform's menu

/** What the page asked of the shell, and what it should do next. */
const fake = {
  /**
   * The calls, in order: ["initial"], ["pick", kind], ["build", name], ["setCheck", id, checked]
   * and ["browse", url].
   */
  calls: [],
  /** The check marks as the platform's menu shows them, by command id. */
  checks: {},
  /** What the next picks choose, in order: a source each; nothing is chosen once it is empty. */
  picks: [],
  /** A message the next build rejects with, as a shell does when it cannot read the source. */
  fail: null,
  /** A fixture that the next build returns instead of its source's, as if the file had changed. */
  next: null,
  /** Choose a command in the platform's menu. */
  menu(id) {
    if (handler) handler(id);
  },
};
window.fakeShell = fake;

/** @type {import("../../apps/viewer-ui/shell.js").Shell} */
export default {
  platform,
  nativeMenu: platform === "desktop",
  async initial() {
    fake.calls.push(["initial"]);
    if (params.has("fail-start")) throw params.get("fail-start");
    return source(params.get("open"));
  },
  async pick(kind) {
    fake.calls.push(["pick", kind]);
    return source(fake.picks.shift());
  },
  async build(src) {
    fake.calls.push(["build", src.name]);
    const [fail, name] = [fake.fail, fake.next || src.fixture];
    fake.fail = null;
    fake.next = null;
    // a plain message, as the desktop shell's invoke rejects with
    if (fail !== null) throw fail;
    const response = await fetch(`/fixtures/${encodeURIComponent(name)}.json`);
    if (!response.ok) throw `${src.name}: there is no such fixture`;
    return response.json();
  },
  setCheck(id, checked) {
    fake.calls.push(["setCheck", id, checked]);
    fake.checks[id] = checked;
  },
  onCommand(h) {
    handler = h;
  },
  browse(url) {
    fake.calls.push(["browse", url]);
  },
};
