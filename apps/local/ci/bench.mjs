// Times the solver of the local program: starts each program given, asks it to solve a netlist through its solver service (the
// page's own request, POST /rnl-solver/solve, with the key the program gives), and prints the median of the wall times of the
// solves of each. The programs are tried in turn, round after round, so that what the machine does meanwhile falls on all of them.
//
//   node apps/local/ci/bench.mjs NETLIST SITE_DIR ROUNDS NAME=PROGRAM [NAME=PROGRAM]...
//
// It ends with the ratio of the time of the second program to the first, and fails if MAX_RATIO is set and the ratio is more.

import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

const [, , netlist, site, rounds, ...programs] = process.argv;
if (!netlist || !site || !rounds || programs.length < 2) {
  console.error("usage: bench.mjs NETLIST SITE_DIR ROUNDS NAME=PROGRAM NAME=PROGRAM...");
  process.exit(2);
}
const work = mkdtempSync(join(tmpdir(), "rnl-bench-"));

/** Start a program on the netlist: its port, and the key of its session. */
async function start(program) {
  const child = spawn(program, ["--no-browser", "--site", site, "--settings", join(work, "settings.json"), netlist], {
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, XDG_CONFIG_HOME: join(work, "config"), XDG_CACHE_HOME: join(work, "cache") },
  });
  let text = "";
  const address = await new Promise((ok, fail) => {
    child.stdout.on("data", (chunk) => {
      text += chunk;
      const m = /(http:\/\/127\.0\.0\.1:\d+)\/__start\?t=([0-9a-f]{64})/.exec(text);
      if (m) ok({ base: m[1], token: m[2] });
    });
    child.stderr.on("data", (chunk) => (text += chunk));
    child.on("exit", (code) => fail(new Error(`${program} ended with ${code}: ${text}`)));
  });
  const started = await fetch(`${address.base}/__start?t=${address.token}`, { redirect: "manual" });
  const key = /#k=([0-9a-f]{64})/.exec(started.headers.get("location") ?? "")?.[1];
  if (!key) throw new Error(`${program} gave no key (${started.status})`);
  return { child, base: address.base, key };
}

const body = JSON.stringify({ entry: `/bench/${basename(netlist)}`, files: { [`/bench/${basename(netlist)}`]: readFileSync(netlist, "utf8") }, pool: [], options: "time_limit=none" });

async function solve(server) {
  const began = process.hrtime.bigint();
  const response = await fetch(`${server.base}/rnl-solver/solve`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RNL": "1", "X-RNL-Key": server.key },
    body,
  });
  const text = await response.text();
  if (!response.ok) throw new Error(`the solve answered ${response.status}: ${text.slice(0, 300)}`);
  return Number(process.hrtime.bigint() - began) / 1e9;
}

const servers = [];
for (const spec of programs) {
  const [name, program] = [spec.slice(0, spec.indexOf("=")), spec.slice(spec.indexOf("=") + 1)];
  servers.push({ name, ...(await start(program)), times: [] });
}
// (the first solve of each is not timed: the program and the machine are warm after it)
for (const server of servers) await solve(server);
for (let round = 1; round <= Number(rounds); round++) {
  for (const server of servers) {
    const seconds = await solve(server);
    server.times.push(seconds);
    console.log(`${server.name} round ${round}: ${seconds.toFixed(2)} s`);
  }
}
const median = (list) => [...list].sort((a, b) => a - b)[Math.floor(list.length / 2)];
const medians = servers.map((s) => ({ name: s.name, median: median(s.times) }));
for (const { name, median: m } of medians) console.log(`${name}: median ${m.toFixed(2)} s`);
const ratio = medians[1].median / medians[0].median;
console.log(`${medians[1].name} / ${medians[0].name} = ${ratio.toFixed(2)}`);
const summary = { netlist: basename(netlist), medians, ratio };
writeFileSync(join(work, "bench.json"), JSON.stringify(summary, null, 2));
if (process.env.GITHUB_STEP_SUMMARY) {
  const lines = ["### Solve time of the local program (median of " + rounds + " rounds, " + basename(netlist) + ")", "", "| program | seconds |", "| --- | --- |"];
  for (const { name, median: m } of medians) lines.push(`| ${name} | ${m.toFixed(2)} |`);
  lines.push("", `${medians[1].name} / ${medians[0].name} = **${ratio.toFixed(2)}**`);
  writeFileSync(process.env.GITHUB_STEP_SUMMARY, lines.join("\n") + "\n", { flag: "a" });
}
for (const server of servers) server.child.kill();
const most = Number(process.env.MAX_RATIO ?? "");
if (most && ratio > most) {
  console.error(`${medians[1].name} is ${ratio.toFixed(2)} times as slow as ${medians[0].name}; the most allowed is ${most}`);
  process.exit(1);
}
process.exit(0);
