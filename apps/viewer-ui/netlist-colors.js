// One netlist palette, independent of the UI theme and the drawing's colours, and beside its
// selection highlight the drawing's second one: the colour of a selected subcircuit instance, which
// can be opened. The drawing lightens it on a dark background, as it does its first highlight.
import { append, m, setAttr, setText } from "./i18n.js";
import { form, inform } from "./dialog.js";
import { load, save } from "./util.js";

export const COLOR_FIELDS = [
  ["component", "Component objects (#)", "#2275E2"],
  ["net", "Net objects (&)", "#A01008"],
  ["rnl", "RNL region background", "#E1FEE9"],
  ["keyword", "RNL keywords", "#000000"],
  ["definition", "Definition objects ($)", "#2B7800"],
  ["local", "Local objects (%)", "#4B21E4"],
  ["string", "Strings", "#895D7C"],
  ["parameter", "Parameters and roles", "#1D4781"],
  ["free", "Free text", "#6B6B6B"],
  ["text", "Other text", "#000000"],
  ["background", "Netlist background", "#FFFFFF"],
  ["highlight", "Selection highlight", "#E8590C"],
  ["subcircuit", "Subcircuit selection highlight (drawing)", "#C2255C"],
];
export const DEFAULT_COLORS = Object.freeze(Object.fromEntries(COLOR_FIELDS.map(([key, , value]) => [key, value])));
const KEY = "netlist-colors";
const hex = value => typeof value === "string" && /^#?[0-9a-f]{6}$/i.test(value.trim())
  ? "#" + value.trim().replace(/^#/, "").toUpperCase() : null;
let current = { ...DEFAULT_COLORS };
let busy = false;

function normalized(value) {
  return Object.fromEntries(COLOR_FIELDS.map(([key]) => [key, hex(value?.[key]) || DEFAULT_COLORS[key]]));
}
function apply(value) {
  current = { ...value };
  const root = document.documentElement.style;
  for (const [key, color] of Object.entries(current)) root.setProperty(`--netlist-${key}`, color);
  // The drawing's subcircuit colour and its fill; the stylesheet's own when the default is kept.
  if (current.subcircuit === DEFAULT_COLORS.subcircuit) { root.removeProperty("--sub"); root.removeProperty("--sub-fill"); }
  else {
    root.setProperty("--sub", current.subcircuit);
    root.setProperty("--sub-fill", `color-mix(in srgb, ${current.subcircuit} 6%, transparent)`);
  }
  window.dispatchEvent(new Event("highlight-colors"));
}
export function startNetlistColors() { apply(normalized(load(KEY, null))); }

export async function netlistColors() {
  if (busy) return;
  busy = true;
  const before = { ...current }, draft = { ...current }, fields = [];
  try {
    const rows = document.createElement("div");
    rows.className = "netlist-colors";
    for (const [key, title] of COLOR_FIELDS) {
      const row = document.createElement("div");
      row.className = "netlist-color-row";
      row.dataset.color = key;
      const label = document.createElement("label");
      label.htmlFor = `netlist-color-${key}`;
      setText(label, m(title));
      const text = document.createElement("input");
      text.type = "text";
      text.id = label.htmlFor;
      text.value = draft[key];
      text.maxLength = 7;
      text.spellcheck = false;
      text.autocomplete = "off";
      text.className = "color-hex";
      const picker = document.createElement("input");
      picker.type = "color";
      picker.value = draft[key];
      setAttr(picker, "aria-label", m`${m(title)} color picker`);
      const update = () => {
        const value = hex(text.value);
        text.setCustomValidity(value ? "" : String(m("Enter a six-digit hex color, such as #1A67CC.")));
        text.setAttribute("aria-invalid", String(!value));
        if (value) { draft[key] = value; picker.value = value; apply(draft); }
      };
      text.addEventListener("input", update);
      text.addEventListener("change", () => { update(); if (hex(text.value)) text.value = draft[key]; });
      picker.addEventListener("input", () => { text.value = picker.value.toUpperCase(); update(); });
      picker.addEventListener("change", () => { text.value = picker.value.toUpperCase(); update(); });
      fields.push({ key, text, picker });
      append(row, label, text, picker);
      append(rows, row);
    }
    const reset = document.createElement("button");
    reset.type = "button";
    reset.className = "netlist-colors-reset";
    setText(reset, m("Restore Defaults"));
    reset.addEventListener("click", () => {
      Object.assign(draft, DEFAULT_COLORS);
      for (const { key, text, picker } of fields) {
        text.value = picker.value = draft[key];
        text.setCustomValidity(""); text.setAttribute("aria-invalid", "false");
      }
      apply(draft);
    });
    const result = await form({
      title: m("Color Scheme"),
      text: m("Preview changes immediately. Save keeps them; Cancel restores the previous scheme."),
      parts: [rows], footer: [reset], focus: fields[0].text, action: "Save", wide: true,
      answer: () => {
        const invalid = fields.find(({ text }) => !hex(text.value));
        if (invalid) { invalid.text.reportValidity(); invalid.text.focus(); return undefined; }
        return { ...draft };
      },
    });
    if (!result) { apply(before); return; }
    try {
      await save(KEY, result);
      if (JSON.stringify(normalized(load(KEY, null))) !== JSON.stringify(result)) throw new Error(String(m("Settings storage is unavailable.")));
      apply(result);
    } catch (error) {
      apply(before);
      await inform({ title: m("Color Scheme"), paragraphs: [m`Could not save color scheme: ${error.message || error}`] });
    }
  } finally {
    busy = false;
  }
}
