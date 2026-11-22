import { m, tr } from "../i18n.js";
// The local shell: the page served by the local program (apps/local) to a browser, on the machine the program runs on, or,
// over `ssh -L`, to a browser on another. It is the desktop viewer, for a machine where the desktop app cannot run: the same
// interface and the same rules (platform "desktop"), with the files of the machine of the program behind it: the program
// lists them for the chooser, reads them for the page, and writes the ones the user saves where they were read from, in the
// encoding they had. What differs is only what a browser cannot do: the menus and dialogs are the page's own, not native;
// the core runs in the page as WebAssembly (the web shell's); and a file dropped on the page, which a browser gives without
// its path, is not opened. None of the web's own is here: no URL, no manifest, no public packages online.
// Solve is the web shell's, which finds the program's solver service. The settings are kept by the program, in a file of the
// user's own, not in the browser: its storage is of one address, and the program's address changes with every run.
//
// A file of the machine is a source whose path is its path on that machine, so that Document paths, diagnostics and Save say
// the same as on the desktop; the page reads it, and the files it includes, from `/f/<path>`, which the program serves below
// the folders it was told to allow (the folder of each file opened, and the one above it).

import web, { useBundledPackages } from "./web.js";
import { decodeText } from "../core/client.js";
import { chooseOnServer } from "../server-files.js";
import { inform, offer } from "../dialog.js";
import { sayListedInPart } from "../incoming.js";
import { sourcePath } from "../source-path.js";
import { doc } from "../state.js";
import { dirtyPaths, unappliedPaths } from "../edits.js";

/**
 * The program's own API, as the page of the program may call it: with the header it asks of every request that does
 * anything (the key of the session goes with every request: local-key.js). `body` is sent as JSON, `bytes` as they are.
 */
async function api(path, { method = "GET", body, bytes } = {}) {
  const type = bytes !== undefined ? "application/octet-stream" : body !== undefined ? "application/json" : null;
  const response = await fetch(new URL(path, location.href), {
    method: bytes !== undefined ? "POST" : method,
    cache: "no-store",
    credentials: "same-origin",
    headers: { "X-RNL": "1", ...(type ? { "Content-Type": type } : {}) },
    body: bytes !== undefined ? bytes : body === undefined ? undefined : JSON.stringify(body),
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

/** The public packages bundled beside the program, as the desktop's: none in the program alone. */
const publicPackages = () => api("api/public-packages");
// their files, read once, for the core in the page to find packages in by name (web.js useBundledPackages)
useBundledPackages(publicPackages().then(async (info) => {
  const files = {};
  for (const p of info.packages || []) files[p.path] = decodeText(await fileBytes(p.path));
  return { files, dirs: info.dirs || [] };
}));

const encodePath = (path) => path.split("/").map(encodeURIComponent).join("/");
/** Where the program serves a file of the machine, by its absolute path. */
const address = (path) => new URL(`f${encodePath(path)}`, location.href).href;
/** Whether a workspace path is a file of the machine: rooted, and not one of the page's own virtual places. */
const onMachine = (path) => typeof path === "string" && path.startsWith("/") && !path.startsWith("/<");

// the files the page has read from the machine, by their paths: the program can name them again, at the next start too
const machineFiles = new Set();

/** A file's bytes, as the program serves it; rejects when it does not (it is not there, or not below an allowed folder). */
async function fileBytes(path) {
  const response = await fetch(address(path), { cache: "no-store", credentials: "same-origin" });
  if (!response.ok) throw new Error(`${path} could not be read (${response.status}).`);
  return new Uint8Array(await response.arrayBuffer());
}

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
  machineFiles.add(path);
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

// A browser gives a file dropped on the page without its path, which the desktop's rules need (Save writes a file where it
// is): the page says so, and opens nothing.
document.addEventListener("drop", (e) => {
  if (![...(e.dataTransfer?.types || [])].includes("Files")) return;
  void inform({ title: m("Open"), paragraphs: [m("A browser does not tell the page where a dropped file is. Open it with File › Open.")] });
});

// what the web shell has that is the web's own: none of it here (a URL, a manifest, public packages online, the Viewer API)
const {
  includeLibrary: _library, publicPackages: _published, includePackage: _include, externalFile: _external,
  pickManifest: _pickManifest, sourceFromManifestJson: _manifestJson, saveManifest: _saveManifest,
  // (the web's limit is of a page that holds a project's texts in its core; the desktop has none, nor has this)
  projectLimits: _limits, ...shared
} = web;

/**
 * Bytes the page made (a project's archive, an image), written by the program as the file the user chose in the chooser
 * of the machine's files, as Save As writes one: a file that is there is replaced only when the user says so. What was
 * written, or null when the user cancelled.
 */
async function writeChosen({ name, start, title }, make) {
  const chosen = await chooseOnServer(api, { kind: "save", start, name, title, action: tr("Save"), remember });
  if (!chosen) return null;
  // (the bytes are made once the place is known, as the desktop makes them after its dialog)
  const bytes = await make();
  const at = (overwrite) => `api/write?${new URLSearchParams({ to: chosen.path, ...(overwrite ? { overwrite: "1" } : {}) })}`;
  let written = await api(at(false), { bytes });
  if (written.exists) {
    const replace = await offer({ title, paragraphs: [m`${written.exists} exists already. Replace it?`], action: tr("Replace") });
    if (!replace) return null;
    written = await api(at(true), { bytes });
  }
  return written.path;
}

const folderOf = (path) => (path ? path.slice(0, path.lastIndexOf("/")) || "/" : "");
// the archive of the project saved or opened last, whose folder and name Save Project offers again, as the desktop does
let lastProject = null;
// the folder the user chose to open a project in, ending in "/" (projectRoot)
let projectFolder = null;
// the last solve shown: whether its copy is one file, and the file solved (Save Solved Copy As)
let solved = null;

/** @type {import("../shell.js").Shell} */
export default {
  ...shared,
  platform: "desktop",
  /** The file named on the command line, the first of them; else none. */
  initial() {
    return Promise.resolve(session.initial.length > 0 ? serverSource(session.initial[0]) : null);
  },
  /** The page opens nothing it is given otherwise: a file comes by File › Open or the command line (and see the drop above). */
  onOpen() {},
  /**
   * The session of a new window (New Window, in a new tab of the browser): its files of the machine are read from the
   * machine again, as the desktop's new window reads them from disk, with the texts applied and not saved over them
   * (`edits`, navigation.js); the web shell's handoff carries the rest.
   */
  async initialSession() {
    const session = await web.initialSession();
    if (!session) return null;
    const machine = (source) => {
      if (!source || source.temporary || !onMachine(source.path)) return source;
      const read = serverSource(source.path);
      return source.view ? { ...read, entrySource: read, view: source.view } : read;
    };
    session.entry = machine(session.entry);
    session.path = session.path.map((step) => ({ ...step, entry: machine(step.entry) }));
    session.pool = session.pool.map((item) => (onMachine(item.path) ? { ...item, ...web.poolItem(serverSource(item.path)) } : item));
    return session;
  },
  /** The chooser of the machine's files. */
  async pick(kind) {
    if (kind !== "file") return null;
    const chosen = await chooseOnServer(api, { kind: "file", start: lastDir, browser: null, remember });
    if (!chosen) return null;
    await api("api/open", { method: "POST", body: { path: chosen.path } });
    return serverSource(chosen.path);
  },
  /** Include File, and Include Folder: from the machine of the program. */
  async include(kind) {
    if (kind !== "file" && kind !== "folder") return [];
    const chosen = await chooseOnServer(api, {
      kind: kind === "folder" ? "folder" : "file",
      start: lastDir,
      title: kind === "folder" ? m("Choose Folder") : undefined,
      browser: null,
      action: kind === "folder" ? tr("Choose") : undefined,
      remember,
    });
    if (!chosen) return [];
    if (kind === "file") {
      await api("api/open", { method: "POST", body: { path: chosen.path } });
      return [web.poolItem(serverSource(chosen.path))];
    }
    // the program allows the folder and lists it as the desktop's Include Folder does (rnl_cli::walk), by the core's suffixes
    const listed = await api("api/folder", { method: "POST", body: { dir: chosen.path } });
    // (a folder with more files than are listed is included in part: said, as every shell says it)
    sayListedInPart(listed);
    return listed.files.map((path) => web.poolItem(serverSource(path)));
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
    return saved.changed?.length ? { changed: saved.changed } : { written: saved.written };
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
  /** Add PATH: a folder of the machine of the program, chosen as Include Folder's is. */
  async pickPath() {
    const chosen = await chooseOnServer(api, { kind: "folder", start: lastDir, title: m("Choose Folder"), browser: null, action: tr("Choose"), remember });
    return chosen?.path ?? null;
  },
  /**
   * The packages and netlists of a PATH folder, as lookup finds them there, and with `subfolders` those of the folders
   * below it: the program lists a folder only when the settings have it as PATH (and with its sub-folders, for those).
   */
  pathPackages: (dir, subfolders = false) => api(`api/path?${new URLSearchParams({ dir, ...(subfolders ? { subfolders: "1" } : {}) })}`),
  /** The public packages bundled beside the program, which the Library lists and Help › Public Packages says, as the desktop's. */
  bundledPackages: publicPackages,
  /** The bytes of a file of the machine, such as a PATH file that Save Project holds (project.js). */
  readFile: fileBytes,
  /** `text` as bytes in the encoding of the file of the machine at `path`, as Save writes it (project.js, for an edited file). */
  async encodeLike(path, text) {
    const response = await fetch(new URL("api/encode-like", location.href), {
      method: "POST", cache: "no-store", credentials: "same-origin",
      headers: { "X-RNL": "1", "Content-Type": "application/json" }, body: JSON.stringify({ path, text }),
    });
    if (!response.ok) throw new Error(`the program answered ${response.status} ${response.statusText}`);
    return new Uint8Array(await response.arrayBuffer());
  },
  /** Save Project, as the desktop's: the archive written where the user says, on the machine of the program. */
  async saveProject(name, build) {
    const offered = lastProject ? lastProject.split("/").pop() : name;
    const path = await writeChosen({ name: offered, start: folderOf(lastProject) || lastDir, title: m("Save Project") },
      async () => new Uint8Array(await (await build()).arrayBuffer()));
    if (!path) return false;
    lastProject = path;
    return path;
  },
  /** Open Project, as the desktop's: an archive of the machine of the program. */
  async pickProject() {
    const chosen = await chooseOnServer(api, { kind: "file", start: folderOf(lastProject) || lastDir, title: m("Open Project"), browser: null, remember });
    if (!chosen) return null;
    const { path } = await api("api/open", { method: "POST", body: { path: chosen.path } });
    return { name: path.split("/").pop(), path, bytes: await fileBytes(path) };
  },
  /** The folder to open a project in, which the user chooses; files of the project already there are replaced only when the user says. */
  async projectRoot(picked, manifest) {
    const chosen = await chooseOnServer(api, { kind: "folder", start: folderOf(picked.path) || lastDir, title: m("Choose the folder to open the project in"), browser: null, action: tr("Choose"), remember });
    if (!chosen) return null;
    const { dir } = await api("api/project-folder", { method: "POST", body: { dir: chosen.path } });
    const { existing } = await api("api/project-existing", { method: "POST", body: { paths: manifest.files.map((f) => f.path) } });
    if (existing.length) {
      const shown = existing.slice(0, 5).join(", ") + (existing.length > 5 ? m`, and ${existing.length - 5} more` : "");
      const replace = await offer({ title: m("Open Project"), paragraphs: [m`The folder already has files of this project: ${shown}.`, m("Replace them?")], action: tr("Replace") });
      if (!replace) return null;
    }
    projectFolder = `${dir.replace(/\/$/, "")}/`;
    return projectFolder;
  },
  /** Write a project read into its folder, on the machine of the program: its entry, and its pool as Library files. */
  async projectSource(read, picked) {
    for (const [path, bytes] of Object.entries(read.bytes)) {
      if (!projectFolder || !path.startsWith(projectFolder)) throw new Error(`${path} is not in the folder chosen for the project.`);
      await api(`api/project-file?${new URLSearchParams({ path: path.slice(projectFolder.length) })}`, { bytes });
    }
    lastProject = picked.path ?? lastProject;
    return { source: serverSource(read.entry), items: read.pool.map((path) => web.poolItem(serverSource(path))) };
  },
  /** Export Image, as the desktop's: the image written where the user says. */
  saveImage: (name, blob) =>
    writeChosen({ name, start: folderOf(doc?.path) || lastDir, title: m("Export Image") }, async () => new Uint8Array(await blob.arrayBuffer())),
  /** Solve, as the web shell's with the program's solver; the program keeps the solve, for Save Solved Copy As. */
  async solve(source, settings, edits) {
    const answer = await web.solve(source, settings, edits);
    solved = { oneFile: answer.report?.one_file === true, entry: answer.document?.path ?? doc?.path ?? "" };
    return answer;
  },
  /**
   * Save Solved Copy As, as the desktop's: the program saves the copy of the last solve it kept, as one file where the
   * user says, or into a folder the user chooses, with the files it read beside it; never over a file it read.
   */
  async saveSolved(encoding) {
    if (!solved) throw "solve the netlist first: there is no solve to save";
    const name = solved.entry.split("/").pop() || "netlist";
    const dot = name.lastIndexOf(".");
    const copy = dot > 0 ? `${name.slice(0, dot)}-solved${name.slice(dot)}` : `${name}-solved`;
    const start = folderOf(solved.entry) || lastDir;
    const chosen = solved.oneFile
      ? await chooseOnServer(api, { kind: "save", start, name: copy, title: m("Save Solved Copy As"), action: tr("Save"), remember })
      : await chooseOnServer(api, { kind: "folder", start, title: m("Save the Solved Files Into a Folder"), browser: null, action: tr("Choose"), remember });
    if (!chosen) return null;
    // (a folder chosen is allowed as one included is: the copy is written into it)
    if (!solved.oneFile) await api("api/allow", { method: "POST", body: { dir: chosen.path } });
    const body = { to: chosen.path, encoding: encoding ?? "preserve" };
    let saved = await api("api/save-solved", { method: "POST", body });
    if (saved.exists) {
      const replace = await offer({ title: m("Save Solved Copy As"), paragraphs: [m`${saved.exists} exists already. Replace it?`], action: tr("Replace") });
      if (!replace) return null;
      saved = await api("api/save-solved", { method: "POST", body: { ...body, overwrite: true } });
    }
    return { to: saved.to, written: saved.written, copied: saved.copied };
  },
  /**
   * What changed on disk since the version `since` (disk-changes.js): the program looks at the files the page read. The
   * page asks once a second, which tells the program that it is still open, and how many files have changes saved
   * nowhere (the program does not stop while there are).
   */
  fileChanges: (since) => api(`api/changes?since=${since}&unsaved=${new Set([...dirtyPaths(), ...unappliedPaths()]).size}`),
  /** A format chosen for a file of the machine is kept for the next start, as the desktop keeps it. */
  keepsFormat: (path) => machineFiles.has(path),
  /** A file of a PATH folder, read from the machine as a file opened is. */
  pathItem(path) {
    const source = { ...serverSource(path), library: true };
    return { source, read: web.poolItem(source).read };
  },
  // the settings are the program's: in a file of the user's own
  loadSettings: async () => api("api/settings"),
  saveSetting: (key, value) => save(key, value).then(() => undefined),
};
