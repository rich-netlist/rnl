import { m, tr } from "./i18n.js";
// Windows requires the user to confirm defaults in its own UI.
import { inform, modal, offer } from "./dialog.js";
import { shell } from "./shell.js";

export async function fileAssociations() {
  if (!shell.fileAssociations || modal()) return;
  const open = await offer({
    title: m("File Associations"),
    paragraphs: [
      m("The Windows installer registers RNL Viewer for .sp, .spi, .spice, .cir, .ckt, .net, .netlist, .inc, .lib, .scs and .rnl files."),
      m("In Windows Settings, find RNL Viewer under Apps > Default apps and choose the file types to open with it."),
      m("For a standalone EXE, right-click a netlist in Explorer, choose Open with > Choose another app, browse to RNL Viewer, and select Always."),
    ],
    action: tr("Open Default Apps"),
  });
  if (!open) return;
  try { await shell.fileAssociations(); }
  catch (error) {
    await inform({ title: m("File Associations"), paragraphs: [m`Windows Settings could not be opened: ${error.message || error}`] });
  }
}
