// Explicit UI messages only. Netlists, filenames, SVG content and core diagnostics stay raw.
// The English source is the catalog key and fallback; message values never carry action identity.
// The languages and their catalogs come from i18n/ of the repository (decision 0015): the shell
// gives them (uiLanguages, uiCatalog), or the page reads them from locales/, where the web viewer's
// build writes them (scripts/i18n/viewer-locales.mjs).
let language = "en";
// the languages offered, English first: { code, lang, name, english, matches } as viewer-locales.mjs gives them
let languages = [{ code: "en", lang: "en", name: "English", english: "English", matches: ["en"] }];
const catalogs = {}; // the catalog of each language but English, by its code
let host = null;
let persist = null;
let switching = Promise.resolve();
const texts = new WeakMap();
const attributes = new WeakMap();

class Message {
  constructor(key, values = []) { this.key = key; this.values = values; }
  toString() { return translate(this.key, this.values); }
}

/** Mark a UI string, or a template whose substituted values must remain literal. A string may have
 * placeholders {0}, {1}… for the values after it, as a template's ${…} are. */
export function m(source, ...values) {
  if (source instanceof Message) return source;
  if (Array.isArray(source)) return new Message(source.map((part, i) => part + (i < values.length ? `{${i}}` : "")).join(""), values);
  return new Message(String(source), values);
}

/** Mark a UI string that is translated where it is shown, by m() of a variable, such as the label of a
 * dialog's button or a name in a table: the scan of the catalog finds it here (scripts/i18n/status.py). */
export const tr = (source) => source;

export function translate(key, values = []) {
  const catalog = catalogs[language];
  const pattern = catalog && typeof catalog[key] === "string" ? catalog[key] : key;
  return pattern.replace(/\{(\d+)\}/g, (match, index) => index < values.length ? String(values[index]) : match);
}
export const locale = () => language;
/** The languages offered, English first, each { code, lang, name, matches }. */
export const uiLanguages = () => languages;
/** The language the browser or the system asks for first, if it is offered, else English. */
export function preferredLanguage(wanted = navigator.languages) {
  return languageFor(wanted?.[0])?.code || "en";
}
/** The language offered that a browser or system language, such as "zh-TW", asks for, or undefined: of
 * those whose `matches` fit it, the one that fits it longest, in whatever order the registry lists them,
 * so that zh-TW takes a Traditional Chinese that names zh-tw rather than the zh of Simplified Chinese,
 * as the site's root does (site/build.py). */
export function languageFor(wanted = "") {
  const tag = wanted.toLowerCase();
  let found, length = -1;
  for (const l of languages) {
    for (const prefix of l.matches) {
      if ((tag === prefix || tag.startsWith(`${prefix}-`)) && prefix.length > length) [found, length] = [l, prefix.length];
    }
  }
  return found;
}
/** Whether a text is in a catalog, and so shown translated in some language. */
const translatable = (key) => Object.values(catalogs).some((catalog) => typeof catalog[key] === "string");

function textNode(value) {
  const node = document.createTextNode(String(value ?? ""));
  if (value instanceof Message) texts.set(node, value);
  return node;
}

/** Bind the text node, not its parent: later appended inputs and links survive a language change. */
export function setText(node, value) { node.replaceChildren(textNode(value)); }
export function append(node, ...values) { node.append(...values.map(value => value instanceof Message ? textNode(value) : value)); }
export function setAttr(node, name, value) {
  let bindings = attributes.get(node);
  if (!bindings) attributes.set(node, bindings = new Map());
  if (value instanceof Message) bindings.set(name, value); else bindings.delete(name);
  if (value == null) node.removeAttribute(name);
  else node.setAttribute(name, String(value));
}

// Called once before opening any document: only the application's static HTML exists here.
function bindStatic() {
  const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  while (walk.nextNode()) {
    const node = walk.currentNode;
    if (node.nodeType === Node.TEXT_NODE && !node.parentElement.closest("script,style,svg")) {
      const key = node.data.trim();
      if (translatable(key)) {
        const value = m`${node.data.match(/^\s*/)[0]}${m(key)}${node.data.match(/\s*$/)[0]}`;
        texts.set(node, value); node.data = String(value);
      }
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      for (const attr of ["title", "placeholder", "aria-label"]) {
        const value = node.getAttribute(attr);
        if (value && translatable(value)) setAttr(node, attr, m(value));
      }
    }
  }
}

function refresh() {
  document.documentElement.lang = language;
  const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  while (walk.nextNode()) {
    const node = walk.currentNode;
    const message = texts.get(node);
    if (message) node.data = String(message);
    for (const [name, value] of attributes.get(node) || []) {
      if (node.hasAttribute(name)) node.setAttribute(name, String(value));
      else attributes.get(node).delete(name);
    }
  }
  window.dispatchEvent(new Event("language-changed"));
}

/** A file of locales/ that the web viewer's build writes. */
async function locales(name) {
  const response = await fetch(`locales/${name}`);
  if (!response.ok) throw new Error(`Cannot load UI translations (${response.status}).`);
  return response.json();
}

/**
 * Take the languages and their catalogs, and show the interface in the saved language, else the one the
 * system asks for, else English. A language whose catalog cannot be read is left out, and the page
 * starts without it; without the list of languages, it starts in English alone (decision 0015: what
 * is missing shows in English). Resolves to a line that says what was left out, or null: in English in
 * every language, as every notice about translation itself, naming a language by its English name, and
 * so not a text of the catalogs. The desktop app builds its catalogs in, and never leaves one out.
 */
export async function startI18n(shell, saved, save) {
  host = shell;
  persist = save;
  const english = languages[0];
  let unavailable = null;
  try {
    languages = await (host.uiLanguages ? host.uiLanguages() : locales("languages.json"));
  } catch (error) {
    console.warn(`The languages of the interface could not be read: ${error.message || error}`);
    unavailable = "The interface's languages could not be loaded; it is shown in English.";
  }
  // every catalog, as the static HTML is bound to the texts that any of them translates
  const failed = [];
  await Promise.all(languages.slice(1).map(async (l) => {
    try {
      catalogs[l.code] = await (host.uiCatalog ? host.uiCatalog(l.code) : locales(`${l.code}.json`));
    } catch (error) {
      console.warn(`The interface's catalog of ${l.code} could not be read: ${error.message || error}`);
      failed.push(l);
    }
  }));
  if (failed.length) {
    languages = languages.filter((l) => !failed.includes(l));
    unavailable = `The interface could not load ${failed.map((l) => l.english || l.name).join(", ")}; it is offered in the other languages.`;
  }
  if (!languages.length || languages[0].code !== english.code) languages = [english, ...languages.filter((l) => l.code !== english.code)];
  language = languages.some((l) => l.code === saved) ? saved : preferredLanguage();
  await host.setLanguage?.(language);
  bindStatic();
  refresh();
  return unavailable;
}

/** Serialize native-menu updates and settings writes when users switch quickly. */
export function setLanguage(next) {
  if (!languages.some((l) => l.code === next)) return Promise.reject(new Error("Unsupported UI language"));
  const change = async () => {
    await host?.setLanguage?.(next);
    language = next;
    refresh();
    await persist?.(next);
  };
  switching = switching.catch(() => {}).then(change);
  return switching;
}
