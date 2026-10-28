// One protocol for iframe canvases and the complete web viewer. UI adapters own presentation.
import { viewerChannel } from "./window-session.js";
import { shell } from "./shell.js";
import { sourceFromManifest, sourceFromUrl } from "./shells/web.js";
import { API_FEATURES, EMBED_FEATURES, MAX_OPEN_BYTES } from "./viewer-api-contract.js";

function validateOpen(message) {
  const bytes = new TextEncoder().encode(JSON.stringify(message)).byteLength;
  if (bytes > MAX_OPEN_BYTES) throw new Error(`open is ${bytes} bytes; the limit is ${MAX_OPEN_BYTES} bytes (64 MiB).`);
  if (message.requestId !== undefined && (typeof message.requestId !== "string" || !message.requestId.length || message.requestId.length > 128))
    throw new Error("requestId must be a nonempty string of at most 128 characters.");
  const packages = message.packages ?? [];
  if (!Array.isArray(packages) || packages.length > 64 || packages.some(p => !p ||
      typeof p.name !== "string" || !/^[a-z0-9-]{1,128}$/.test(p.name) || typeof p.version !== "string" || !/^\d+\.\d+$/.test(p.version)))
    throw new Error("packages must contain at most 64 package names and exact major.minor versions.");
  let source;
  if (typeof message.text === "string" && typeof message.name === "string" && message.name.trim() && message.name.length <= 1024)
    source = { name: message.name, text: message.text, temporary: true };
  else if (typeof message.url === "string" && typeof message.manifest === "string") throw new Error("open takes url or manifest, not both.");
  else if (typeof message.url === "string") source = sourceFromUrl(message.url);
  else if (typeof message.manifest === "string") source = sourceFromManifest(message.manifest);
  else throw new Error("open needs text and name, url, or manifest.");
  return { source, packages };
}

export function startViewerApi({ version, opener = false, begin = () => null, open, command = () => {}, onError = () => {} }) {
  const channel = viewerChannel(opener);
  let pending = null;
  const send = channel.send;
  function fail(request, reason, message) {
    if (request.done) return;
    request.done = true;
    request.controller.abort();
    send("error", { ...request.fields, reason, message });
  }
  function supersede(request) {
    if (request.fields.requestId !== undefined) fail(request, "superseded", "superseded");
    else { request.done = true; request.controller.abort(); } // legacy callers expect superseded opens to stay silent
  }
  async function receiveOpen(message, initial = false) {
    const fields = typeof message.requestId === "string" ? { requestId: message.requestId } : {};
    const request = { fields, done: false, controller: new AbortController() };
    let input;
    try { input = validateOpen(message); }
    catch (error) { fail(request, "invalid", String(error.message || error)); return; }
    if (pending && !pending.done) {
      supersede(pending);
    }
    pending = request;
    const current = () => pending === request && !request.done;
    const operation = begin(request.controller.signal);
    operation?.addEventListener("abort", () => { if (!request.done) supersede(request); }, { once: true });
    try {
      if (!current()) return;
      const packages = (await Promise.all(input.packages.map(async p => {
        try { return await shell.includePackage(p); }
        catch (error) { throw new Error(`${p.name} ${p.version}: ${error.message || error}`); }
      }))).flat();
      if (!current()) return;
      const result = await open(input.source, packages, current, initial, message, request.controller.signal);
      if (!current()) return;
      request.done = true;
      send("loaded", { ...fields, ...result });
    } catch (error) {
      if (!current()) return;
      const message = `Could not open the netlist: ${error.message || error}`;
      fail(request, error.reason === "declined" ? "declined" : "open-failed", error.reason === "declined" ? "declined" : message);
      if (error.reason !== "declined") onError(message);
    }
  }
  const stop = channel.listen(message => {
    if (message.type === "open") { void receiveOpen(message); return; }
    try { Promise.resolve(command(message)).catch(error => send("error", { message: String(error.message || error) })); }
    catch (error) { send("error", { message: String(error.message || error) }); }
  });
  send("ready", { version, features: [...API_FEATURES.filter(f => opener || f !== "opener"), ...(opener ? [] : EMBED_FEATURES)], maxOpenBytes: MAX_OPEN_BYTES });
  window.addEventListener("pagehide", stop, { once: true });
  return { send, open: receiveOpen };
}
