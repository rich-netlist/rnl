// The cache of the core's results by file (result-cache.js): keys, the memory kept within its limit and given again only
// while the inputs a result records hold, and the site's IndexedDB store, which another page finds, which is emptied when
// it was written by another format, and which keeps within its limit too. Each test has a store of its own.

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
  expect(seen.empty).toMatchObject({ entries: 0, used: 0, limit: 0, persistent: false, hits: 3, misses: 2 });
});

test("in the site's store: another page finds a result, a store of another format is emptied, and it keeps within its limit", async ({ page }) => {
  const name = `results-test-${test.info().project.name}-${test.info().workerIndex}-${Date.now()}`;
  const seen = await inPage(page, `
    const [name] = arg;
    const one = new rc.ResultCache({ store: await rc.openStore({ core: "0.6.1", shape: 1 }, name) });
    const value = { levels: [{ title: "top", svg: "<svg/>".repeat(200) }], diagnostics: [] };
    await one.put("k1", value, [["body", "/p/top.sp", "h1"]]);
    // another page, with a memory of its own, finds it in the store, with its inputs
    const two = new rc.ResultCache({ store: await rc.openStore({ core: "0.6.1", shape: 1 }, name) });
    let inputs = null;
    const found = await two.get("k1", async (given) => { inputs = given; return true; });
    const persistent = two.stats().persistent;
    // a store written by another core is emptied when it is opened
    const other = new rc.ResultCache({ store: await rc.openStore({ core: "0.6.2", shape: 1 }, name) });
    const afterUpgrade = await other.get("k1");
    // within the limit: the least recently used goes first
    const small = new rc.ResultCache({ memoryLimit: 0, storeLimit: 1e9, store: await rc.openStore({ core: "0.6.2", shape: 1 }, name) });
    const noise = (n) => ({ noise: Array.from({ length: 400 }, (_, k) => (k * 7919 + n * 104729) % 1000003) });
    await small.put("a", noise(1), []);
    await small.put("b", noise(2), []);
    await small.put("c", noise(3), []);
    const each = small.stats().stored / 3;
    await small.get("a");
    await small.setLimits({ storeLimit: Math.ceil(each * 2.5) });
    const left = [await small.get("a"), await small.get("b"), await small.get("c")].map((v) => v !== null);
    await small.clear();
    const cleared = await small.get("a");
    indexedDB.deleteDatabase((await import("./storage-prefix.js")).STORAGE_PREFIX + name);
    return { found, inputs, persistent, afterUpgrade, left, cleared };
  `, [name]);
  expect(seen.persistent).toBe(true);
  expect(seen.found.levels[0].title).toBe("top");
  expect(seen.found.levels[0].svg.length).toBe(1200);
  expect(seen.inputs).toEqual([["body", "/p/top.sp", "h1"]]);
  expect(seen.afterUpgrade).toBeNull();
  // "a" was used after "b" and "c" were written, so "b", the least recently used, went
  expect(seen.left).toEqual([true, false, true]);
  expect(seen.cleared).toBeNull();
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
