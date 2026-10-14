// Help > Check for Updates, on a platform whose shell has `version` and `browse` (the desktop): the
// newest release of the viewer on GitHub against the version that runs, in a dialog that offers to
// open the release's page. It looks only when the user asks, never by itself. The shell says the
// version and opens the page in the default browser (shell.js); the list of releases comes from
// GitHub's API.

import { inform, modal, offer } from "./dialog.js";
import { shell } from "./shell.js";

/** The repository's releases, newest first: the language standard's as well as the viewer's. */
export const RELEASES_API = "https://api.github.com/repos/rich-netlist/rnl/releases?per_page=30";
/** The page that lists them; a release's own page is under it, at tag/ and its tag. */
export const RELEASES = "https://github.com/rich-netlist/rnl/releases";
/** The prefix of the viewer's tags, followed by its version, such as viewer-v0.1.0. */
const TAG = "viewer-v";
/** How long GitHub may take to answer (ms). */
const WAIT = 15000;
const TITLE = "Check for Updates";

let checking = false;

/** Look for a newer release and say what was found; nothing while a dialog is shown or a look is under way. */
export async function checkForUpdates() {
  if (!shell.version || !shell.browse || checking || modal()) return;
  checking = true;
  document.body.classList.add("checking");
  let found;
  try {
    const [running, releases] = await Promise.all([shell.version(), fetchReleases()]);
    found = { running, newest: newestRelease(releases) };
  } catch (e) {
    found = { why: e instanceof Error ? e.message : String(e) };
  } finally {
    checking = false;
    document.body.classList.remove("checking");
  }
  if (found.why !== undefined) {
    await inform({ title: TITLE, paragraphs: [`The latest version could not be looked up: ${found.why}.`, releasesLink()] });
    return;
  }
  const { running, newest } = found;
  if (!newest) {
    await inform({ title: TITLE, paragraphs: ["GitHub lists no release of RNL Viewer yet.", releasesLink()] });
    return;
  }
  // a version that is not one, which the app's never is, comes before every release
  const order = compareVersions(newest.parsed, parseVersion(running) || { core: [0, 0, 0], pre: [0] });
  if (order > 0) {
    const open = await offer({
      title: TITLE,
      paragraphs: [
        heading(`RNL Viewer ${newest.version} is available.`),
        `This is version ${running}. The release's page on GitHub has its installers and what changed.`,
      ],
      action: "Open Release Page",
    });
    if (open) openPage(newest.url);
    return;
  }
  const text = order === 0
    ? `RNL Viewer ${running} is the latest version.`
    : `RNL Viewer ${running} is newer than the latest release, ${newest.version}.`;
  await inform({ title: TITLE, paragraphs: [heading(text)] });
}

/**
 * GitHub's list of the repository's releases; rejects with an Error that says why it could not be
 * had, for the dialog to finish its sentence with.
 */
async function fetchReleases() {
  const stop = new AbortController();
  const timer = setTimeout(() => stop.abort(), WAIT);
  try {
    let response;
    try {
      response = await fetch(RELEASES_API, {
        headers: { Accept: "application/vnd.github+json" },
        cache: "no-store",
        signal: stop.signal,
      });
    } catch {
      throw new Error(stop.signal.aborted ? `GitHub did not answer in ${WAIT / 1000} seconds` : "GitHub could not be reached");
    }
    if (!response.ok) {
      // GitHub answers so many requests from one address an hour without an account
      if (response.headers.get("x-ratelimit-remaining") === "0") {
        throw new Error("GitHub takes no more requests from this network for a while");
      }
      throw new Error(`GitHub answered ${`${response.status} ${response.statusText}`.trim()}`);
    }
    const releases = await response.json().catch(() => null);
    if (!Array.isArray(releases)) throw new Error("GitHub's answer is not a list of releases");
    return releases;
  } finally {
    clearTimeout(timer);
  }
}

/**
 * The newest release of the viewer among GitHub's releases: of those tagged viewer-v and a version,
 * neither drafts nor prereleases, the one with the highest version, and the address of its page;
 * null when there is none.
 *
 * @param {{ tag_name?: string, draft?: boolean, prerelease?: boolean }[]} releases
 * @returns {{ version: string, parsed: Version, url: string } | null}
 */
export function newestRelease(releases) {
  let newest = null;
  for (const r of releases) {
    if (!r || r.draft || r.prerelease || typeof r.tag_name !== "string" || !r.tag_name.startsWith(TAG)) continue;
    const version = r.tag_name.slice(TAG.length);
    const parsed = parseVersion(version);
    if (parsed && (!newest || compareVersions(parsed, newest.parsed) > 0)) {
      newest = { version, parsed, url: `${RELEASES}/tag/${encodeURIComponent(r.tag_name)}` };
    }
  }
  return newest;
}

/**
 * A version as Semantic Versioning 2.0.0 orders it: its major, minor and patch numbers, and its
 * prerelease identifiers, each a number or a text.
 *
 * @typedef {{ core: number[], pre: (number | string)[] }} Version
 */

/** A version written as Semantic Versioning has it, such as "1.2.3-rc.1+build.5"; null when it is not one. */
export function parseVersion(text) {
  const m = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.exec(text);
  if (!m) return null;
  const pre = m[4] ? m[4].split(".").map((id) => (/^(0|[1-9]\d*)$/.test(id) ? Number(id) : id)) : [];
  return { core: [Number(m[1]), Number(m[2]), Number(m[3])], pre };
}

/**
 * Which of two versions comes first: negative when `a` does, positive when `b` does, 0 when they are
 * equal. The numbers decide first; then a version with prerelease identifiers comes before the one
 * without, and the identifiers decide one by one: numbers by value and before texts, texts in ASCII
 * order, and fewer identifiers before more. Build metadata does not count.
 *
 * @param {Version} a
 * @param {Version} b
 */
export function compareVersions(a, b) {
  for (let k = 0; k < 3; k += 1) if (a.core[k] !== b.core[k]) return a.core[k] - b.core[k];
  if (a.pre.length === 0 || b.pre.length === 0) return b.pre.length - a.pre.length;
  for (let k = 0; k < Math.min(a.pre.length, b.pre.length); k += 1) {
    const [x, y] = [a.pre[k], b.pre[k]];
    if (x === y) continue;
    if (typeof x !== typeof y) return typeof x === "number" ? -1 : 1;
    if (typeof x === "number") return x - y;
    return x < y ? -1 : 1;
  }
  return a.pre.length - b.pre.length;
}

/** Open a page of the repository in the default browser; when that fails, say where it is. */
function openPage(url) {
  Promise.resolve().then(() => shell.browse(url)).catch((e) => {
    inform({ title: TITLE, paragraphs: [`The page could not be opened: ${e instanceof Error ? e.message : String(e)}.`, url] });
  });
}

/** The first paragraph of a dialog, in bold. */
function heading(text) {
  const p = document.createElement("p");
  p.className = "dialog-name";
  p.textContent = text;
  return p;
}

/** A paragraph with a link to the releases' page, which opens in the default browser, not in the page. */
function releasesLink() {
  const a = document.createElement("a");
  a.href = RELEASES;
  a.textContent = RELEASES.replace(/^https:\/\//, "");
  a.addEventListener("click", (e) => {
    e.preventDefault();
    openPage(RELEASES);
  });
  const p = document.createElement("p");
  p.append("The releases are listed at ", a, ".");
  return p;
}
