// The key of the session with the local program (apps/local), for the page it serves (`?shell=local`): every request of the
// page to that program carries it in the header `X-RNL-Key`, and the program answers nothing that reads or writes the user's
// files, or solves, without it. The program gives it once, in the fragment of the address it goes on to after the one-time
// address (`/?shell=local#k=KEY`): a fragment is not sent to any server, and is taken from the address here. It is kept in
// this tab's `sessionStorage` (so that a reload has it, and a window that this one opens, which gets a copy), not in a cookie:
// a browser sends a cookie to every port of the same address, so a page from any other local port, another user's server or
// another `ssh -L` tunnel, would have been handed it.
//
// This module is the first thing the web shell loads, so that the requests it makes at once, such as the one that asks the
// program for its solver, have the key too. On any page that is not the local program's it does nothing.

const NAME = "rnl-local-key";

let key = "";
if (new URLSearchParams(location.search).get("shell") === "local") {
  try {
    key = sessionStorage.getItem(NAME) ?? "";
  } catch {
    // no storage (a browser that keeps none): the key is held while the page is open
  }
  const given = new URLSearchParams(location.hash.slice(1)).get("k");
  if (given) {
    key = given;
    try {
      sessionStorage.setItem(NAME, key);
    } catch {
      // as above
    }
    history.replaceState(null, "", location.pathname + location.search);
  }
}

if (key) {
  const original = window.fetch.bind(window);
  /** The page's own fetch, with the key added to a request of this origin (the program's own, never another site's). */
  window.fetch = (input, init = {}) => {
    const url = new URL(input instanceof Request ? input.url : String(input), location.href);
    if (url.origin !== location.origin) return original(input, init);
    const headers = new Headers(init.headers ?? (input instanceof Request ? input.headers : undefined));
    headers.set("X-RNL-Key", key);
    return original(input, { ...init, headers });
  };
}
