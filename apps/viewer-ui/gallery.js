import { append, locale, m, setAttr, setText } from "./i18n.js";
// A design of the gallery, opened by ?manifest= (shells/web.js): the slim bar above the drawing that
// says where it comes from, with the way back to the gallery, and the level its manifest names.
import { notice } from "./drawing.js";
import { currentSource } from "./open.js";
import { doc } from "./state.js";
import { enterRoot } from "./subcircuits.js";
import { $ } from "./util.js";

const GALLERY = "https://rich-netlist.github.io/rnl/";

/** Show the bar of `source` when it is a gallery design, or hide it. */
export function showGalleryBar(source) {
  const bar = $("gallery-bar");
  const gallery = source?.gallery;
  bar.hidden = !gallery;
  bar.replaceChildren();
  if (!gallery) return;
  const zh = locale() === "zh-CN";
  const title = (zh && gallery.title?.zh) || gallery.title?.en || gallery.id;
  const name = document.createElement("strong");
  name.className = "gallery-title";
  setText(name, title);
  append(bar, name);
  if (gallery.author) {
    const author = document.createElement("span");
    author.className = "gallery-author";
    setText(author, gallery.author);
    append(bar, author);
  }
  const { repo, commit, url } = gallery.source || {};
  if (repo && commit) {
    const link = document.createElement("a");
    link.className = "gallery-source";
    link.href = url ? `${url.replace(/\/$/, "")}/tree/${commit}` : `https://github.com/${repo}/tree/${commit}`;
    link.target = "_blank";
    link.rel = "noopener";
    setText(link, m`Source: ${repo}@${commit.slice(0, 7)}`);
    setAttr(link, "title", m`${repo} at commit ${commit}`);
    append(bar, link);
  }
  const back = document.createElement("a");
  back.className = "gallery-back";
  back.href = `${GALLERY}${zh ? "zh" : "en"}/gallery.html`;
  setText(back, m("Back to the gallery"));
  append(bar, back);
}

/**
 * Show the level a gallery design names: `?level=` in the address first, then its manifest's;
 * "(top)" is the top level. A name the netlist has no level of is told, and the level shown stays.
 */
export function showGalleryLevel(source) {
  const wanted = new URLSearchParams(location.search).get("level") ?? source?.gallery?.level;
  if (!wanted || !doc) return;
  if (wanted === "(top)") return enterRoot(0);
  const level = doc.levels.findIndex((l) => l.title === wanted);
  if (level < 0) {
    notice(m`This design has no level ${wanted}; the level shown is the default one.`);
    return;
  }
  return enterRoot(level);
}

window.addEventListener("language-changed", () => {
  const bar = document.getElementById("gallery-bar");
  if (bar && !bar.hidden) showGalleryBar(currentSource());
});
