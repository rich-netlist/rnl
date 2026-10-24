import { append, locale, m, setAttr, setText } from "./i18n.js";
// Files opened by an external manifest, ?manifest= (shells/web.js): the slim bar above the drawing
// that says where they come from, with what the manifest gives of its title, author and source, and
// the level the manifest names.
import { notice } from "./drawing.js";
import { doc } from "./state.js";
import { enterRoot } from "./subcircuits.js";
import { $ } from "./util.js";

// the manifest whose bar is shown, for a change of language
let shownExternal = null;

/**
 * The authors a manifest names, `[{ name, url }]`: it gives a list of `{ name, url? }` (or of names),
 * or one name; a url is kept only when it is an https address.
 */
function authors(author) {
  const list = Array.isArray(author) ? author : author ? [author] : [];
  return list.map((a) => (typeof a === "string" ? { name: a } : { name: a?.name, url: a?.url }))
    .filter((a) => typeof a.name === "string" && a.name.trim())
    .map((a) => ({ name: a.name, url: typeof a.url === "string" && /^https:\/\//i.test(a.url) ? a.url : null }));
}

/**
 * Show the bar of `external`, the manifest the file shown belongs to (pool.js), when it gives a
 * title, an author or a source; with null, or a manifest that gives none of them, hide it.
 */
export function showManifestBar(external) {
  const bar = $("manifest-bar");
  shownExternal = external;
  const { repo, commit, url } = external?.source || {};
  const title = external && ((locale() === "zh-CN" && external.title?.zh) || external.title?.en);
  const people = authors(external?.author);
  const shown = !!(title || people.length || (repo && commit));
  bar.hidden = !shown;
  bar.replaceChildren();
  if (!shown) return;
  if (title) {
    const name = document.createElement("strong");
    name.className = "manifest-title";
    setText(name, title);
    append(bar, name);
  }
  if (people.length) {
    // the names in their order, separated by commas; a name with an address links to it
    const line = document.createElement("span");
    line.className = "manifest-author";
    people.forEach((person, k) => {
      if (k) append(line, document.createTextNode(", "));
      const name = document.createElement(person.url ? "a" : "span");
      if (person.url) {
        name.href = person.url;
        name.target = "_blank";
        name.rel = "noopener noreferrer";
      }
      setText(name, person.name);
      append(line, name);
    });
    append(bar, line);
  }
  if (repo && commit) {
    const link = document.createElement("a");
    link.className = "manifest-source";
    // the manifest's url links the files' directory at that commit
    link.href = url || `https://github.com/${repo}/tree/${commit}`;
    link.target = "_blank";
    link.rel = "noopener";
    setText(link, m`Source: ${repo}@${commit.slice(0, 7)}`);
    setAttr(link, "title", m`${repo} at commit ${commit}`);
    append(bar, link);
  }
}

/**
 * Show the level that files opened by a manifest open on: `?level=` in the address first, then the
 * manifest's: a subcircuit's name, or "(top)" for the netlist's own level. A name the netlist has
 * no subcircuit of is told, and the level shown stays.
 */
export function showManifestLevel(source) {
  return showLevel(new URLSearchParams(location.search).get("level") ?? source?.external?.level);
}

/** Show the level named `wanted`, as a manifest names it (a project's too): a subcircuit's name or "(top)". */
export function showLevel(wanted) {
  if (!wanted || !doc) return;
  if (wanted === "(top)") return enterRoot(0, { here: true });
  const level = doc.levels.findIndex((l) => l.kind === "subckt" && l.title === wanted);
  if (level < 0) {
    notice(m`The netlist has no level ${wanted}; the level shown is the default one.`);
    return;
  }
  // in the entry's document, with all it includes, even when an included file defines the subcircuit
  return enterRoot(level, { here: true });
}

window.addEventListener("language-changed", () => {
  if (document.getElementById("manifest-bar")) showManifestBar(shownExternal);
});
