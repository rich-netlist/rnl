// A file of an earlier version of the language, 0.2 or 0.3, which the core reads none of the RNL of (16.1): the page of the
// level shown says so in place of the drawing (drawing.js, and embed.js for an embedded canvas, which refuses a document with
// errors), with the old viewer that reads it. The version is the language version of the file the level is written in: its
// file level's in the outline, or else the core's error RNL2102 version-unsupported on that file; a package file's is not
// the page's to tell. Where the old viewer is depends on the shell (`shell.oldViewer`):
// - "page", the web viewer: the web viewer of the 0.5 series, which the site keeps at viewer-0.3/, opened on the same
//   circuit: the same ?url= or ?manifest= when the document's entry is of an earlier version, else the address of the file
//   itself (the old viewer cannot read the 0.4 entry); a file from this computer cannot be passed by a link, so the link
//   opens the old viewer alone. The page opened by ?url= or ?manifest= goes there in the same tab; a file from this
//   computer, whose files the page holds, and an embedded canvas, whose host drives it, open it in a new tab.
// - "release", the desktop: the page of the release of RNL Viewer 0.5.3, to download it, in the system's browser.
// - "release-and-page", the local program: the same, and the old web viewer, where the file is opened with File › Open
//   (Viewer 0.5.3 has no local program, and its Linux desktop builds may not run where this one does).

import { m } from "./i18n.js";
import { shell } from "./shell.js";
import { entrySource } from "./visit-source.js";

/** The web viewer of the 0.5 series, which reads files of 0.2 and 0.3, as the site keeps it. */
export const OLD_VIEWER = "https://rich-netlist.github.io/rnl/viewer-0.3/";
/** The release of the last viewer that reads them, with its downloads. */
export const OLD_RELEASE = "https://github.com/rich-netlist/rnl/releases/tag/viewer-v0.5.3";
/** The versions the old viewer reads and this one does not. */
const EARLIER = ["0.2", "0.3"];
/** The parameters of the page's address that the old viewer reads too, carried over to it. */
const CARRIED = ["background", "grid", "names", "usages", "currents"];
const UNSUPPORTED = "RNL2102";
// what stands for the link in a sentence, as a language words it: the sentence is split there
const LINK = "\u0001";

/** The language version of the file `path` of the Document `d`: by the outline, else by the core's error on it; null when neither says. */
function versionOf(d, path) {
  const outline = d.outline;
  if (outline) {
    const at = outline.files.findIndex((f) => f.path === path);
    const version = at < 0 ? null : outline.levels.find((l) => l.kind === "file" && l.file === at)?.version;
    if (version) return version;
  }
  const k = d.files.findIndex((f) => f.path === path);
  const error = d.diagnostics.find((x) => x.code === UNSUPPORTED && x.file_index === k && k >= 0);
  return /^unsupported version (\S+):/.exec(error?.message ?? "")?.[1] ?? null;
}

/** The path of the file of the level `lv` of `d`: a subcircuit's, else the file the Document reads (its entry's, or the file shown alone). */
function fileOf(d, lv) {
  if (lv.lines) return d.files[lv.lines[0]]?.path ?? null;
  if (typeof d.file === "string") return d.file;
  return d.files[0]?.path ?? null;
}

/** The path of the document's entry: the outline's first file, else the first the Document reads. */
const entryOf = (d) => d.outline?.files[0]?.path ?? d.files[0]?.path ?? null;

/**
 * Of the level `i` of the Document `d`, read from `source`, when the file it is written in is of an earlier version: that
 * version and the old viewer that reads it (`link`), else null.
 */
export function oldVersion(d, i, source) {
  const lv = d?.levels?.[i];
  const path = lv && fileOf(d, lv);
  if (!path) return null;
  const version = versionOf(d, path);
  return EARLIER.includes(version) ? { version, link: linkFor(d, path, source) } : null;
}

/**
 * Of a document that an embedded canvas refused for its errors: the first of its files of an earlier version, when every
 * error is one of the core's errors of a file of an earlier version, else null.
 */
export function oldDocument(d, source) {
  const errors = d.diagnostics.filter((x) => x.severity === "error");
  if (errors.length === 0 || !errors.every((x) => x.code === UNSUPPORTED && x.file_index !== null)) return null;
  const entry = entryOf(d);
  for (const path of [entry, ...d.files.map((f) => f.path).filter((p) => p !== entry)]) {
    const version = path && versionOf(d, path);
    if (EARLIER.includes(version)) return { version, link: linkFor(d, path, source) };
  }
  return null;
}

/** The workspace path of the file `path` as the Document names it, relative to the folder of the entry at `entry`. */
function joined(entry, path) {
  const names = (path.startsWith("/") ? path : `${entry.slice(0, entry.lastIndexOf("/"))}/${path}`).split("/");
  const out = [];
  for (const name of names) {
    if (name === "" || name === ".") continue;
    if (name === ".." && out.length > 0 && out[out.length - 1] !== "..") out.pop();
    else out.push(name);
  }
  return `/${out.join("/")}`;
}

/** The address of the old viewer with `key` set to `value`, and the parameters of this page's address it reads too. */
function oldViewerAt(key, value) {
  const url = new URL(OLD_VIEWER);
  const here = new URLSearchParams(location.search);
  url.searchParams.set(key, value);
  for (const name of CARRIED) if (here.has(name)) url.searchParams.set(name, here.get(name));
  return url.href;
}

/**
 * Where the old viewer is for the file `path` of `d`, read from `source`: `{ href, sameTab, local }`, `local` when the
 * file came from this computer and is opened there by hand; or `{ release }` with the release page, and `page` too in the
 * local program.
 */
function linkFor(d, path, source) {
  const kind = shell?.oldViewer ?? "page";
  if (kind === "release") return { release: OLD_RELEASE };
  if (kind === "release-and-page") return { release: OLD_RELEASE, page: OLD_VIEWER };
  const entry = entrySource(source);
  const entryOld = EARLIER.includes(versionOf(d, entryOf(d)));
  let href = null;
  if (entry?.external) {
    // a manifest: itself when its entry is of an earlier version, else the address of the file
    if (entryOld && entry.manifest) href = oldViewerAt("manifest", entry.manifest);
    else if (!entryOld) {
      const address = entry.external.urls?.[joined(entry.external.entry, path)];
      if (address) href = oldViewerAt("url", address);
    }
  } else if (entry?.url) {
    const address = entryOld ? entry.url : entry.address?.(joined(entry.path, path));
    if (address) href = oldViewerAt("url", address);
  }
  if (!href) return { href: OLD_VIEWER, sameTab: false, local: true };
  return { href, sameTab: !shell?.embed, local: false };
}

/** A link with `text` to `href`: in a new tab of the browser unless `sameTab`; in the system's browser on the desktop (`shell.browse`). */
function anchor(text, href, sameTab = false) {
  const a = document.createElement("a");
  a.className = "old-version-link";
  a.href = href;
  a.textContent = String(text);
  if (!sameTab) {
    a.target = "_blank";
    a.rel = "noopener noreferrer";
  }
  if (shell?.platform === "desktop" && shell.browse) {
    a.addEventListener("click", (e) => {
      e.preventDefault();
      void Promise.resolve().then(() => shell.browse(href)).catch(() => {});
    });
  }
  // the link is the page's, not the drawing's: a press on it does not begin a pan
  a.addEventListener("pointerdown", (e) => e.stopPropagation());
  return a;
}

/** A sentence of the notice, the link put where `{0}` stands. */
function sentence(text, link) {
  const span = document.createElement("span");
  const [before, after = ""] = String(text).split(LINK);
  span.append(before, link, after);
  return span;
}

let oldShown = false; // whether the level shown is written in a file of an earlier version

/** Whether the level shown is written in a file of an earlier version, which Solve and Respace do not solve (commands.js). */
export const oldLevelShown = () => oldShown;

/** The level shown is (`on`) or is not written in a file of an earlier version (drawing.js). */
export function setOldShown(on) {
  oldShown = on;
}

const shown = new WeakMap(); // the element of a notice -> what it says, to say it again in another language

/** Fill `el` with the notice of `old` (from `oldVersion`): the version, and where the old viewer is. */
function fill(el, old) {
  const { version, link } = old;
  const lines = [document.createTextNode(String(m("Unsupported old file format (RNL {0}).", version)))];
  if (link.release) {
    lines.push(sentence(m("RNL Viewer 0.5.3 opens it: download it from {0}.", LINK), anchor(m("its release page"), link.release)));
    if (link.page) {
      lines.push(sentence(m("Or try {0} in a browser: open the file there with File › Open.", LINK), anchor(m("the old web viewer"), link.page)));
    }
  } else if (link.local) {
    lines.push(sentence(m("You can try {0}: open the file there with File › Open, since a file from this computer cannot be passed to it by a link.", LINK),
      anchor(m("the old viewer"), link.href)));
  } else {
    lines.push(sentence(m("You can try opening it in {0}.", LINK), anchor(m("the old viewer"), link.href, link.sameTab)));
  }
  el.replaceChildren(...lines.flatMap((line, k) => (k ? [document.createElement("br"), line] : [line])));
  shown.set(el, old);
}

/** Show the notice of `old` in the page of the level, `page` (#empty), in place of what it said. */
export function showOldVersion(page, old) {
  const notice = document.createElement("span");
  notice.className = "old-version";
  fill(notice, old);
  page.replaceChildren(notice);
}

window.addEventListener("language-changed", () => {
  for (const notice of document.querySelectorAll(".old-version")) {
    const old = shown.get(notice);
    if (old) fill(notice, old);
  }
});
