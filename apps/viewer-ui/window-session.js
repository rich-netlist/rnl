// Same-origin, one-use handoff. Neither files nor session tokens leave the viewer's origin.
export async function openWebWindow(serialize) {
  const token = crypto.randomUUID();
  const url = new URL(location.href);
  url.searchParams.delete("url");
  url.searchParams.delete("embed");
  url.searchParams.delete("origin");
  url.searchParams.set("viewer-session", token);
  let popup;
  let timer;
  let receive;
  const ready = new Promise((resolve, reject) => {
    receive = (e) => {
      if (e.origin === location.origin && e.source === popup && e.data?.type === "rnl-viewer-ready" && e.data.token === token) resolve();
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
      if (e.origin !== location.origin || e.source !== opener || e.data?.type !== "rnl-viewer-session" || e.data.token !== token) return;
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
