// Small helpers for building the page.

export const $ = (id) => document.getElementById(id);

export function clamp(v, lo, hi) {
  return Math.min(hi, Math.max(lo, v));
}

export function span(cls, text) {
  const s = document.createElement("span");
  s.className = cls;
  s.textContent = text;
  return s;
}

export function svgElement(name, attrs) {
  const el = document.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}
