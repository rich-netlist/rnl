// Help > Public Packages, on the desktop: what public packages are, whether this build bundles them and
// which versions, where they come from, how to add others by hand, and that the viewer never downloads
// one. The desktop finds the bundled ones offline (apps/desktop/src/public_packages.rs).
import { inform } from "./dialog.js";
import { append, m } from "./i18n.js";
import { shell } from "./shell.js";

const SITE = "https://rich-netlist.github.io/rnl-packages/";
const RELEASES = "https://github.com/rich-netlist/rnl/releases";

/** A paragraph of a text with a link in it, which the default browser opens: `before`, the link's text, `after`. */
function linked(before, text, href, after) {
  const p = document.createElement("p");
  const a = document.createElement("a");
  a.href = href;
  append(a, text);
  a.addEventListener("click", (e) => { e.preventDefault(); shell.browse?.(href); });
  append(p, before, a, after);
  return p;
}

export async function bundledPackagesHelp() {
  if (!shell.bundledPackages) return;
  let bundled = { bundled: false, packages: [] };
  try { bundled = await shell.bundledPackages(); } catch { /* what cannot be read is not bundled */ }
  const list = bundled.packages.map((p) => `${p.name} ${p.version}`).join(", ");
  await inform({ title: m("Public Packages"), paragraphs: [
    m("Public packages are RNL packages published on the rnl-packages site: extensions, symbol and style libraries and PDKs, which a netlist uses with @USE-PACKAGE."),
    bundled.bundled
      ? m`This build bundles ${list}. They are found without a network, as a group of their own in the Library, and searched after the directories of your own PATH.`
      : m("This build does not bundle the public packages."),
    // what rnl-packages declares in its REUSE.toml, which the build carries beside the packages (LICENSES/)
    ...(bundled.bundled ? [m("The bundled packages are under CC0-1.0, as rnl-packages declares; their licences are in the LICENSES folder beside them.")] : []),
    bundled.bundled
      ? linked(m("They are the latest versions when this release was made, and change only with a new release of the viewer. A download without them is on the "), m("release page"), RELEASES, ".")
      : linked(m("A download with the public packages bundled is on the "), m("release page"), RELEASES, m(", next to this one.")),
    linked(m("The packages come from the "), m("rnl-packages site"), SITE,
      m(". To use one that this build does not have, download its folder and add the folder to the PATH in the Library; the PATH is searched before the bundled packages.")),
    m("The desktop viewer never downloads packages itself."),
  ] });
}
