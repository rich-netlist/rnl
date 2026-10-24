import { m } from "./i18n.js";
// File > Export Image > Export SVG / Export PNG: the whole of the level shown, as the core draws it
// with the overlays (rnl_view::overlays::svg), not the part in view. Usages and current directions
// are drawn as the page shows them, on or off, each usage shown in its colour; the selection, the
// hover and the grid are not. The background is opaque, light or dark as the page's. The fonts the
// page draws the texts with go into the image, so that a PNG, which the page makes from the SVG,
// has the texts of the SVG and of the drawing on screen.
import { darkColour, isDark } from "./background.js";
import { inform, modal } from "./dialog.js";
import { changes } from "./edits.js";
import { note } from "./editor.js";
import { currentSource, isLoading } from "./open.js";
import { shell } from "./shell.js";
import { current, doc } from "./state.js";
import { overlayChoices } from "./usages.js";

/** A PNG is drawn at twice the SVG's size, and no side longer than this many pixels. */
export const PNG_SCALE = 2;
const PNG_MAX_SIDE = 16384;
const PNG_MAX_PIXELS = 256 * 1024 * 1024;
let busy = false;

/** The default name of the image: the file's name without its extension, and the level's. */
function imageName(kind) {
  const level = doc.levels[current];
  const stem = (doc.name || "drawing").split("/").pop().replace(/\.[^.]*$/, "");
  const name = level.kind === "file" ? stem : `${stem}-${level.title}`;
  return `${name.replace(/[\\/:*?"<>|\u0000-\u001f]+/g, "_")}.${kind}`;
}

/** The overlays to ask the core for: all usages, of which the page then keeps those it shows. */
function overlayOptions(choices) {
  return { usages: choices.usages ? "all" : "none", currents: choices.currents, roles: false };
}

/** Keep the usages the page shows, each in its colour; colour the drawing as on a dark background. */
function asShown(svg, choices) {
  for (const el of svg.querySelectorAll("[data-usage]")) {
    const choice = choices.each.get(Number(el.getAttribute("data-usage")));
    if (!choice?.visible) { el.remove(); continue; }
    if (el.classList.contains("usage-fill")) el.setAttribute("fill", choice.color);
    for (const rect of el.querySelectorAll(":scope > rect[stroke]")) rect.setAttribute("stroke", choice.color);
    for (const text of el.querySelectorAll("text")) text.setAttribute("fill", choice.color);
  }
  if (isDark()) {
    for (const el of svg.querySelectorAll("[stroke], [fill]")) {
      if (el.closest(".usage-overlays, .usage-underlays, .current-overlays")) continue;
      for (const attr of ["stroke", "fill"]) {
        const colour = el.getAttribute(attr);
        if (colour) el.setAttribute(attr, darkColour(colour === "white" ? "#ffffff" : colour === "black" ? "#000000" : colour));
      }
    }
  }
  // the paper: the page's drawing background, opaque
  const paper = getComputedStyle(document.body).getPropertyValue("--drawing-background").trim() || "#ffffff";
  const backdrop = svg.querySelector("rect.backdrop");
  if (backdrop) backdrop.setAttribute("fill", paper);
  else {
    const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    const [x, y, w, h] = (svg.getAttribute("viewBox") || "0 0 0 0").split(/\s+/);
    Object.entries({ class: "backdrop", x, y, width: w, height: h, fill: paper }).forEach(([k, v]) => rect.setAttribute(k, v));
    svg.insertBefore(rect, svg.firstChild);
  }
}

/** Whether a CSS unicode-range takes any of the characters `codes`. */
function inRange(codes, range) {
  const spans = range.split(",").map((part) => {
    const [from, to] = part.trim().replace(/^U\+/i, "").split("-");
    if (from.includes("?")) return [parseInt(from.replace(/\?/g, "0"), 16), parseInt(from.replace(/\?/g, "F"), 16)];
    return [parseInt(from, 16), parseInt(to ?? from, 16)];
  });
  return [...codes].some((c) => spans.some(([a, b]) => c >= a && c <= b));
}

function base64(bytes) {
  let text = "";
  for (let k = 0; k < bytes.length; k += 0x8000) text += String.fromCharCode(...bytes.subarray(k, k + 0x8000));
  return btoa(text);
}

/**
 * The page's @font-face rules for the families the drawing names, those whose ranges its texts use,
 * with their fonts as data, so that the image draws its texts as the page does wherever it is shown.
 * None when the page has no fonts of its own: then the image, like the page, uses the system's.
 */
async function embeddedFonts(svg) {
  const families = new Set();
  // the core names the drawing's fonts on the <svg> itself, the overlays' on their texts
  for (const el of [svg, ...svg.querySelectorAll("[font-family]")]) {
    if (!el.hasAttribute("font-family")) continue;
    for (const f of el.getAttribute("font-family").split(",")) families.add(f.trim().replace(/^["']|["']$/g, ""));
  }
  const codes = new Set();
  for (const text of svg.querySelectorAll("text, tspan")) for (const c of text.textContent) codes.add(c.codePointAt(0));
  const rules = [];
  for (const sheet of document.styleSheets) {
    let list;
    try { list = sheet.cssRules; } catch { continue; }
    for (const rule of list) if (rule instanceof CSSFontFaceRule) rules.push({ rule, base: sheet.href || location.href });
  }
  let css = "";
  for (const { rule, base } of rules) {
    const family = rule.style.getPropertyValue("font-family").replace(/["']/g, "").trim();
    if (!families.has(family)) continue;
    const range = rule.style.getPropertyValue("unicode-range");
    if (range && !inRange(codes, range)) continue;
    const src = /url\(\s*["']?([^"')]+)["']?\s*\)/.exec(rule.style.getPropertyValue("src"))?.[1];
    if (!src) continue;
    const response = await fetch(new URL(src, base));
    if (!response.ok) continue;
    const type = response.headers.get("content-type")?.split(";")[0] || "font/woff2";
    const data = `data:${type};base64,${base64(new Uint8Array(await response.arrayBuffer()))}`;
    css += `@font-face{font-family:"${family}";src:url(${data});font-weight:${rule.style.getPropertyValue("font-weight") || "normal"};`
      + `font-style:${rule.style.getPropertyValue("font-style") || "normal"};${range ? `unicode-range:${range};` : ""}}\n`;
  }
  if (!css) return;
  const style = document.createElementNS("http://www.w3.org/2000/svg", "style");
  style.textContent = css;
  svg.insertBefore(style, svg.firstChild);
}

/** The SVG of the level shown, as the page shows it. */
async function levelSvg() {
  const choices = overlayChoices();
  const level = doc.levels[current];
  const text = await shell.exportSvg({ doc, level: current, title: level.title, overlays: overlayOptions(choices),
    // the texts applied to the Document shown
    source: currentSource(), edits: changes(true) });
  const parsed = new DOMParser().parseFromString(text, "image/svg+xml");
  const svg = parsed.documentElement;
  if (svg.nodeName !== "svg") throw new Error(m("The core gave no drawing for this level."));
  asShown(svg, choices);
  await embeddedFonts(svg);
  return svg;
}

/** The PNG of `svg`, drawn by the browser at PNG_SCALE times its size. */
async function rasterize(svg) {
  const width = Number.parseFloat(svg.getAttribute("width")) || Number.parseFloat(svg.getAttribute("viewBox")?.split(/\s+/)[2]);
  const height = Number.parseFloat(svg.getAttribute("height")) || Number.parseFloat(svg.getAttribute("viewBox")?.split(/\s+/)[3]);
  if (!(width > 0 && height > 0)) throw new Error(m("The drawing has no size."));
  const scale = Math.min(PNG_SCALE, PNG_MAX_SIDE / width, PNG_MAX_SIDE / height, Math.sqrt(PNG_MAX_PIXELS / (width * height)));
  svg.setAttribute("width", String(width));
  svg.setAttribute("height", String(height));
  const url = URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(svg)], { type: "image/svg+xml" }));
  try {
    const image = new Image();
    image.src = url;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(width * scale));
    canvas.height = Math.max(1, Math.round(height * scale));
    canvas.getContext("2d").drawImage(image, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, "image/png"));
    if (!blob) throw new Error(m("The browser could not make the PNG."));
    return blob;
  } finally {
    URL.revokeObjectURL(url);
  }
}

/** File > Export Image > Export SVG (`kind` "svg") or Export PNG ("png"). */
export async function exportImage(kind) {
  if (!shell.exportSvg || !shell.saveImage || busy || modal() || isLoading()) return;
  const title = kind === "png" ? m("Export PNG") : m("Export SVG");
  if (!doc || !doc.levels[current]?.svg) {
    await inform({ title, paragraphs: [m("Open a netlist first.")] });
    return;
  }
  busy = true;
  try {
    note(m("Drawing the image…"), 0);
    const name = imageName(kind);
    const svg = await levelSvg();
    const blob = kind === "png" ? await rasterize(svg)
      : new Blob([`<?xml version="1.0" encoding="UTF-8"?>\n${new XMLSerializer().serializeToString(svg)}\n`], { type: "image/svg+xml" });
    const saved = await shell.saveImage(name, blob, kind);
    note(saved ? m`Exported ${saved}.` : "");
  } catch (error) {
    note("");
    await inform({ title, paragraphs: [m`Could not export the image: ${error.message || error}`] });
  } finally {
    busy = false;
  }
}
