import { m, tr } from "../i18n.js";
// The local shell: the page served by the local program (apps/local) to a browser, on the machine the program runs on, or,
// over `ssh -L`, to a browser on another. It is the web shell, with the core as WebAssembly in the browser, and with the files
// of the machine of the program behind it: the program lists them for the chooser, reads them for the page, and writes the
// ones the user saves where they were read from, in the encoding they had (Save is Save, not Download File: `overwrites`).
// Solve is the web shell's, which finds the program's solver service. The settings are kept by the program, in a file of the
// user's own, not in the browser: its storage is of one address, and the program's address changes with every run.
//
// A file of the machine is a source whose path is its path on that machine, so that Document paths, diagnostics and Save say
// the same as on the desktop; the page reads it, and the files it includes, from `/f/<path>`, which the program serves below
// the folders it was told to allow (the folder of each file opened, and the one above it).

import web from "./web.js";
import { decodeText } from "../core/client.js";
import { chooseOnServer } from "../server-files.js";
import { offer } from "../dialog.js";
import { note } from "../editor.js";
import { sourcePath } from "../source-path.js";
import { doc } from "../state.js";

/** The program's own API, as the page of the program may call it: with the header it asks of every request that does anything (the key of the session goes with every request: local-key.js). */
async function api(path, { method = "GET", body } = {}) {
  const response = await fetch(new URL(path, location.href), {
    method,
    cache: "no-store",
    credentials: "same-origin",
    headers: { "X-RNL": "1", ...(body === undefined ? {} : { "Content-Type": "application/json" }) },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  let json = null;
  try {
    json = await response.json();
  } catch {
    // no JSON: the status says it
  }
  if (!response.ok) throw new Error(json?.error ?? `the program answered ${response.status} ${response.statusText}`);
  return json;
}

const session = await api("api/session");

const encodePath = (path) => path.split("/").map(encodeURIComponent).join("/");
/** Where the program serves a file of the machine, by its absolute path. */
const address = (path) => new URL(`f${encodePath(path)}`, location.href).href;
/** Whether a workspace path is a file of the machine: rooted, and not one of the page's own virtual places. */
const onMachine = (path) => typeof path === "string" && path.startsWith("/") && !path.startsWith("/<");

/** A file's text, or null when the program does not serve it (it is not there, or not below an allowed folder); `why` says. */
async function readFile(path, again) {
  let response;
  try {
    response = await fetch(address(path), { cache: again ? "reload" : "no-store", credentials: "same-origin" });
  } catch {
    throw new Error("the program that serves the files cannot be reached; has it been stopped?");
  }
  if (response.status === 404 || response.status === 403) {
    const why = await response.json().then((j) => j.error, () => "");
    return { text: null, why };
  }
  if (!response.ok) throw new Error(`${path} could not be read (${response.status}).`);
  return { text: decodeText(await response.arrayBuffer()) };
}

/**
 * A file of the machine as the page opens it: named by its path, read when asked and each time again for Reload, with the
 * files it includes read the same way, by their paths.
 */
export function serverSource(path) {
  return {
    name: path,
    path,
    // read at once, while the Library is indexed and the core loads, not when they are done (the web shell's build)
    prefetch: true,
    async open(again) {
      const got = await readFile(path, again);
      if (got.text === null) throw new Error(`${path} could not be read${got.why ? `: ${got.why}` : "."}`);
      return { path, text: got.text };
    },
    async read(file, again) {
      if (!onMachine(file)) return null;
      return (await readFile(file, again)).text;
    },
  };
}

// the folder the chooser starts in: that of the file named on the command line, then the one last chosen from
let lastDir = session.initial[0] ? session.initial[0].slice(0, session.initial[0].lastIndexOf("/")) || "/" : session.home || "";
const remember = (dir) => {
  lastDir = dir;
  save("local-last-folder", dir).catch(() => {});
};
/** Keep a setting in the program's file. */
const save = (key, value) => api("api/settings", { method: "PUT", body: { key, value } });

const NETLISTS = /\.(sp|spi|spice|cir|ckt|net|netlist|scs|inc|lib|rnl)$/i;

/** The netlists and packages in a folder, and in the folders below it, as the Library's Include Folder lists them. */
async function folderFiles(dir, found = [], depth = 0) {
  const listing = await api(`api/ls?${new URLSearchParams({ dir })}`);
  for (const entry of listing.entries) {
    const path = `${listing.dir.replace(/\/$/, "")}/${entry.name}`;
    if (entry.dir) {
      if (depth < 8 && found.length < 2000) await folderFiles(path, found, depth + 1);
    } else if (NETLISTS.test(entry.name)) found.push(path);
  }
  return found;
}

/** The files of the machine that changed on disk since the page read them: said once each, with the key that reloads them. */
let version = 0;
async function watchChanges() {
  try {
    const { version: now, changed } = await api(`api/changes?since=${version}`);
    version = now;
    if (doc && changed.length) {
      const shown = new Set(doc.files.map((f) => sourcePath(doc, f)));
      const mine = changed.filter((p) => shown.has(p));
      if (mine.length) note(m`${mine[0].split("/").pop()} changed on disk. F5 reloads the netlist.`);
    }
  } catch {
    // the program is gone, or has not answered yet: the next time
  }
  setTimeout(watchChanges, 1000);
}
watchChanges();

/** @type {import("../shell.js").Shell} */
export default {
  ...web,
  overwrites: true,
  /** The files named on the command line, the first of them; else what the web shell opens (a ?url= address). */
  initial() {
    if (session.initial.length > 0) return Promise.resolve(serverSource(session.initial[0]));
    return web.initial();
  },
  /** The chooser of the machine's files, whose last entry is the browser's own: the files of the computer the browser is on. */
  async pick(kind) {
    if (kind !== "file") return web.pick(kind);
    const chosen = await chooseOnServer(api, { kind: "file", start: lastDir, browser: () => web.pick("file"), remember });
    if (!chosen) return null;
    if (chosen.browser !== undefined) return chosen.browser;
    await api("api/open", { method: "POST", body: { path: chosen.path } });
    return serverSource(chosen.path);
  },
  /** Include File, and Include Folder: from the machine of the program, or, last in the list, from the browser's computer. */
  async include(kind) {
    if (kind !== "file" && kind !== "folder") return web.include(kind);
    const chosen = await chooseOnServer(api, {
      kind: kind === "folder" ? "folder" : "file",
      start: lastDir,
      title: kind === "folder" ? m("Choose Folder") : undefined,
      browser: kind === "file" ? () => web.include(kind) : null,
      action: kind === "folder" ? tr("Choose") : undefined,
      remember,
    });
    if (!chosen) return [];
    if (chosen.browser !== undefined) return chosen.browser;
    if (kind === "file") {
      await api("api/open", { method: "POST", body: { path: chosen.path } });
      return [web.poolItem(serverSource(chosen.path))];
    }
    await api("api/allow", { method: "POST", body: { dir: chosen.path } });
    return (await folderFiles(chosen.path)).map((path) => web.poolItem(serverSource(path)));
  },
  // another file of the document, opened from the Library, a diagnostic or an include: read from the machine too
  sourceForFile(source, document, index) {
    const full = sourcePath(document, document.files[index]);
    return onMachine(full) ? serverSource(full) : web.sourceForFile(source, document, index);
  },
  sourceForPath: (source, full) => (onMachine(full) ? serverSource(full) : web.sourceForPath(source, full)),
  /** Write the files where they were read from, as the desktop does (decision 0011): asked about one that changed on disk. */
  async save(source, files, overwrite, encoding) {
    const saved = await api("api/save", {
      method: "POST",
      body: {
        entry: source?.path ?? files[0].path,
        files: files.map(({ path, name, text, read }) => ({ path, name, text, read })),
        overwrite: overwrite === true,
        encoding: encoding ?? "preserve",
      },
    });
    return saved.changed?.length ? { changed: saved.changed } : saved.said;
  },
  /** Save As: where the user says, on the machine of the program; the file written is the netlist from then on. */
  async saveAs(source, file, encoding) {
    const chosen = await chooseOnServer(api, {
      kind: "save",
      start: lastDir,
      name: file.name.split("/").pop(),
      title: m("Save As"),
      action: tr("Save"),
      remember,
    });
    if (!chosen) return null;
    const body = { source: onMachine(source?.path) ? source.path : null, to: chosen.path, text: file.text, encoding: encoding ?? "preserve" };
    let written = await api("api/save-as", { method: "POST", body });
    if (written.exists) {
      // a file of that name is there: nothing was written, and the user says whether it is to be replaced
      const replace = await offer({ title: m("Save As"), paragraphs: [m`${written.exists} exists already. Replace it?`], action: tr("Replace") });
      if (!replace) return null;
      written = await api("api/save-as", { method: "POST", body: { ...body, overwrite: true } });
    }
    await api("api/open", { method: "POST", body: { path: written.path } });
    return serverSource(written.path);
  },
  // the settings are the program's: in a file of the user's own
  loadSettings: async () => api("api/settings"),
  saveSetting: (key, value) => save(key, value).then(() => undefined),
};
