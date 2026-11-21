// The cache of the core's results by file (result-cache.js): keys, and the memory kept within its limit and given again
// only while the inputs a result records hold. Nothing is kept in the browser's storage, and the store that earlier
// versions kept in the site's IndexedDB is deleted.

import { expect, test } from "../viewer.js";

test.beforeEach(async ({ viewer }) => {
  await viewer.open();
});

/** Run `body` in the page with the module as `rc`; `body` is the text of an async function's body. */
const inPage = (page, body, arg) => page.evaluate(async ([body, arg]) => {
  const rc = await import("./result-cache.js");
  return new Function("rc", "arg", `return (async () => { ${body} })();`)(rc, arg);
}, [body, arg]);

test("a key is the same for the same parts however they were built, and differs when any part does", async ({ page }) => {
  const keys = await inPage(page, `
    return [
      await rc.cacheKey({ core: "0.6.1", file: "/a/top.sp", options: { svg: true, flylines: false } }),
      await rc.cacheKey({ options: { flylines: false, svg: true }, file: "/a/top.sp", core: "0.6.1" }),
      await rc.cacheKey({ core: "0.6.1", file: "/a/top.sp", options: { svg: true, flylines: true } }),
      await rc.cacheKey({ core: "0.6.2", file: "/a/top.sp", options: { svg: true, flylines: false } }),
    ];`);
  expect(keys[0]).toMatch(/^[0-9a-f]{64}$/);
  expect(keys[1]).toBe(keys[0]);
  expect(new Set(keys).size).toBe(3);
});

test("in memory: a result is given while its inputs hold, dropped when they do not, and the least recently used goes first", async ({ page }) => {
  const seen = await inPage(page, `
    // four results of 300 characters fit, and a fifth does not
    const cache = new rc.ResultCache({ memoryLimit: 2600 });
    const holds = new Set(["a", "b"]);
    const valid = async (inputs) => inputs.every((i) => holds.has(i));
    const big = (n) => ({ text: "x".repeat(n) });
    await cache.put("one", big(300), ["a"]);
    await cache.put("two", big(300), ["b"]);
    const first = [(await cache.get("one", valid))?.text.length, (await cache.get("two", valid))?.text.length];
    holds.delete("b");
    const dropped = await cache.get("two", valid);
    const afterDrop = cache.stats().entries;
    // "one" is used after "three" is written, so "three" is the least recently used when a fifth comes
    await cache.put("three", big(300), []);
    await cache.get("one", valid);
    await cache.put("four", big(300), []);
    await cache.put("five", big(300), []);
    await cache.put("six", big(300), []);
    const kept = [...cache.memory.keys()];
    // a result of more than a quarter of the limit is not kept in memory
    await cache.put("huge", big(600), []);
    const huge = await cache.get("huge");
    // a limit of 0 keeps nothing
    await cache.setLimits({ memoryLimit: 0 });
    return { first, dropped, afterDrop, kept, huge, empty: cache.stats() };
  `);
  expect(seen.first).toEqual([300, 300]);
  expect(seen.dropped).toBeNull();
  expect(seen.afterDrop).toBe(1);
  expect(seen.kept).toEqual(["one", "four", "five", "six"]);
  expect(seen.huge).toBeNull();
  expect(seen.empty).toEqual({ kind: "readings", entries: 0, used: 0, limit: 0, hits: 3, misses: 2 });
});

test("the store of results an earlier version kept in the site's IndexedDB is deleted, and nothing is stored", async ({ page }) => {
  const seen = await inPage(page, `
    const { STORAGE_PREFIX } = await import("./storage-prefix.js");
    const name = STORAGE_PREFIX + "results-test-" + Math.random().toString(16).slice(2);
    const done = (r) => new Promise((resolve, reject) => { r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error); });
    // a store as an earlier version left it
    const opening = indexedDB.open(name, 1);
    opening.onupgradeneeded = () => opening.result.createObjectStore("entries", { keyPath: "key" });
    (await done(opening)).close();
    const named = async () => (await indexedDB.databases()).some((d) => d.name === name);
    const before = await named();
    rc.removeOldStore(name.slice(STORAGE_PREFIX.length));
    await new Promise((resolve) => setTimeout(resolve, 200));
    const after = await named();
    // a cache keeps what it is given in memory alone
    const cache = new rc.ResultCache();
    await cache.put("one", { n: 1 }, []);
    return { before, after, found: await cache.get("one"), stats: cache.stats() };
  `);
  expect(seen.before).toBe(true);
  expect(seen.after).toBe(false);
  expect(seen.found).toEqual({ n: 1 });
  expect(Object.keys(seen.stats).sort()).toEqual(["entries", "hits", "kind", "limit", "misses", "used"]);
});

test("peek looks at a result without using it: no hit or miss, no order changed, nothing dropped", async ({ page }) => {
  const seen = await inPage(page, `
    const cache = new rc.ResultCache({ memoryLimit: 100000 });
    let holds = true;
    const valid = async () => holds;
    await cache.put("one", { n: 1 }, ["a"]);
    await cache.put("two", { n: 2 }, ["b"]);
    const looked = await cache.peek("one", valid);
    const order = [...cache.memory.keys()];
    holds = false;
    const stale = await cache.peek("two", valid);
    const anyway = await cache.peek("two");
    const none = await cache.peek("three");
    return { looked, order, stale, anyway, none, stats: cache.stats() };
  `);
  expect(seen.looked).toEqual({ n: 1 });
  expect(seen.order).toEqual(["one", "two"]);
  expect(seen.stale).toBeNull();
  expect(seen.anyway).toEqual({ n: 2 });
  expect(seen.none).toBeNull();
  expect(seen.stats).toMatchObject({ entries: 2, hits: 0, misses: 0 });
});
