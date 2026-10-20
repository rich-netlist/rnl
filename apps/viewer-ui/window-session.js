// Window identity checks shared by same-origin sessions and the explicit cross-origin API.
function fromWindow(event, peer, origin) {
  return event.source === peer && (origin === "*" || event.origin === origin);
}

/** The iframe parent, or an explicitly enabled opener connection with a one-use launch token. */
export function viewerChannel(opener = false) {
  const params = new URLSearchParams(location.search);
  const origin = params.get("origin") || "*";
  const token = opener ? params.get("token") : null;
  if (origin !== "*" || opener) {
    let parsed;
    try { parsed = new URL(origin); } catch { /* checked below */ }
    if (!parsed || !["http:", "https:"].includes(parsed.protocol) || parsed.origin !== origin)
      throw new Error("The Viewer API needs an exact HTTP(S) origin.");
  }
  const peer = opener ? window.opener : window.parent;
  if (!peer || opener && (!token || !/^[a-zA-Z0-9-]{16,128}$/.test(token)))
    throw new Error("The Viewer API needs its original window and a launch token. Check the caller's COOP policy.");
  return {
    send(type, fields = {}) { peer.postMessage({ rnl: 1, type, ...fields, ...(token ? { token } : {}) }, origin); },
    listen(receive) {
      const handler = (event) => {
        if (!fromWindow(event, peer, origin) || event.data?.rnl !== 1 || token && event.data.token !== token) return;
        receive(event.data);
      };
      window.addEventListener("message", handler);
      return () => window.removeEventListener("message", handler);
    },
  };
}

// Same-origin, one-use session handoff.
export async function openWebWindow(serialize) {
  const token = crypto.randomUUID();
  const url = new URL(location.href);
  url.searchParams.delete("url");
  url.searchParams.delete("embed");
  url.searchParams.delete("origin");
  url.searchParams.delete("api");
  url.searchParams.delete("token");
  url.searchParams.set("viewer-session", token);
  let popup;
  let timer;
  let receive;
  const ready = new Promise((resolve, reject) => {
    receive = (e) => {
      if (fromWindow(e, popup, location.origin) && e.data?.type === "rnl-viewer-ready" && e.data.token === token) resolve();
    };
    window.addEventListener("message", receive);
    timer = setTimeout(() => reject(new Error("The new viewer did not become ready. Try New Window again.")), 30000);
  });
  try {
    popup = window.open(url.href, "_blank");
    if (!popup) throw new Error("The browser blocked the new viewer tab. Allow pop-ups for this site and try again.");
    const [session] = await Promise.all([serialize(), ready]);
    popup.postMessage({ type: "rnl-viewer-session", token, session }, location.origin);
  } finally {
    clearTimeout(timer);
    window.removeEventListener("message", receive);
  }
}

export function receiveWebSession() {
  const url = new URL(location.href);
  const token = url.searchParams.get("viewer-session");
  if (!token) return Promise.resolve(null);
  if (!window.opener) return Promise.reject(new Error("The original viewer is no longer available. Open the file in this tab."));
  return new Promise((resolve, reject) => {
    const opener = window.opener;
    const timer = setTimeout(() => { cleanup(); reject(new Error("The original viewer did not send its session.")); }, 30000);
    const receive = (e) => {
      if (!fromWindow(e, opener, location.origin) || e.data?.type !== "rnl-viewer-session" || e.data.token !== token) return;
      cleanup();
      url.searchParams.delete("viewer-session");
      history.replaceState(null, "", url.href);
      window.opener = null;
      resolve(e.data.session);
    };
    function cleanup() { clearTimeout(timer); window.removeEventListener("message", receive); }
    window.addEventListener("message", receive);
    opener.postMessage({ type: "rnl-viewer-ready", token }, location.origin);
  });
}
