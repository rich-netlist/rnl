// Explicit UI messages only. Netlists, filenames, SVG content and core diagnostics stay raw.
// The English source is the catalog key and fallback; message values never carry action identity.
let language = "en";
let chinese = {};
let host = null;
let persist = null;
let switching = Promise.resolve();
const texts = new WeakMap();
const attributes = new WeakMap();

class Message {
  constructor(key, values = []) { this.key = key; this.values = values; }
  toString() { return translate(this.key, this.values); }
}

/** Mark a UI string, or a template whose substituted values must remain literal. */
export function m(source, ...values) {
  if (source instanceof Message) return source;
  if (Array.isArray(source)) return new Message(source.map((part, i) => part + (i < values.length ? `{${i}}` : "")).join(""), values);
  return new Message(String(source));
}

export function translate(key, values = []) {
  const pattern = language === "zh-CN" && Object.hasOwn(chinese, key) ? chinese[key] : key;
  return pattern.replace(/\{(\d+)\}/g, (match, index) => index < values.length ? String(values[index]) : match);
}
export const locale = () => language;
export const preferredLanguage = (languages = navigator.languages) => /^zh\b/i.test(languages?.[0] || "") ? "zh-CN" : "en";

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
      if (Object.hasOwn(chinese, key)) {
        const value = m`${node.data.match(/^\s*/)[0]}${m(key)}${node.data.match(/\s*$/)[0]}`;
        texts.set(node, value); node.data = String(value);
      }
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      for (const attr of ["title", "placeholder", "aria-label"]) {
        const value = node.getAttribute(attr);
        if (value && Object.hasOwn(chinese, value)) setAttr(node, attr, m(value));
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

export async function startI18n(shell, saved, save) {
  const response = await fetch("locales/zh-CN.json");
  if (!response.ok) throw new Error(`Cannot load UI translations (${response.status}).`);
  chinese = await response.json();
  host = shell;
  persist = save;
  language = ["en", "zh-CN"].includes(saved) ? saved : preferredLanguage();
  await host.setLanguage?.(language);
  bindStatic();
  refresh();
}

/** Serialize native-menu updates and settings writes when users switch quickly. */
export function setLanguage(next) {
  if (!["en", "zh-CN"].includes(next)) return Promise.reject(new Error("Unsupported UI language"));
  const change = async () => {
    await host?.setLanguage?.(next);
    language = next;
    refresh();
    await persist?.(next);
  };
  switching = switching.catch(() => {}).then(change);
  return switching;
}
