import { append, locale, m, setAttr, setText } from "./i18n.js";
// Files opened by an external manifest, ?manifest= (shells/web.js): the slim bar above the drawing
// that says where they come from, with what the manifest gives of its title, author and source, and
// the level the manifest names.
import { notice } from "./drawing.js";
import { currentSource } from "./open.js";
import { doc } from "./state.js";
import { enterRoot } from "./subcircuits.js";
import { $ } from "./util.js";

/** Show the bar of `source` when it was opened by a manifest that gives a title, an author or a source; else hide it. */
export function showManifestBar(source) {
  const bar = $("manifest-bar");
  const external = source?.external;
  const { repo, commit, url } = external?.source || {};
  const title = external && ((locale() === "zh-CN" && external.title?.zh) || external.title?.en);
  const shown = !!(title || external?.author || (repo && commit));
  bar.hidden = !shown;
  bar.replaceChildren();
  if (!shown) return;
  if (title) {
    const name = document.createElement("strong");
    name.className = "manifest-title";
    setText(name, title);
    append(bar, name);
  }
  if (external.author) {
    const author = document.createElement("span");
    author.className = "manifest-author";
    setText(author, external.author);
    append(bar, author);
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
  const wanted = new URLSearchParams(location.search).get("level") ?? source?.external?.level;
  if (!wanted || !doc) return;
  if (wanted === "(top)") return enterRoot(0);
  const level = doc.levels.findIndex((l) => l.kind === "subckt" && l.title === wanted);
  if (level < 0) {
    notice(m`The netlist has no level ${wanted}; the level shown is the default one.`);
    return;
  }
  return enterRoot(level);
}

window.addEventListener("language-changed", () => {
  const bar = document.getElementById("manifest-bar");
  if (bar && !bar.hidden) showManifestBar(currentSource());
});
