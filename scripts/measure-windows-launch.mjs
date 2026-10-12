import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

if (process.platform !== "win32") throw new Error("Windows launch measurement requires Windows");

const repository = fs.realpathSync(path.resolve(import.meta.dirname, ".."));
const executable = insideRepository(argument("--executable") ?? "target/release/Axvaris.exe");
const output = insideRepository(argument("--output") ?? "docs/performance-0.4.2-launch.json");
const iterations = Number(argument("--iterations") ?? 10);
const timeoutMs = Number(argument("--timeout-ms") ?? 20_000);
const probeRoot = path.join(repository, "target", "launch-probe");

if (!Number.isInteger(iterations) || iterations < 1 || iterations > 50) {
  throw new Error("--iterations must be an integer from 1 to 50");
}
if (!fs.existsSync(executable)) throw new Error(`Executable does not exist: ${executable}`);

function argument(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function insideRepository(value) {
  const resolved = path.resolve(repository, value);
  const relative = path.relative(repository, resolved);
  if (!relative || relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`Path must stay inside the repository: ${resolved}`);
  }
  return resolved;
}

function percentile(values, fraction) {
  const ordered = [...values].sort((left, right) => left - right);
  return ordered[Math.max(0, Math.ceil(ordered.length * fraction) - 1)];
}

async function launchOnce(executablePath, dataDirectory, readyFile) {
  fs.mkdirSync(dataDirectory, { recursive: true });
  fs.rmSync(readyFile, { force: true });
  const started = process.hrtime.bigint();
  const child = spawn(executablePath, [], {
    env: {
      ...process.env,
      SABICHEIRO_QA_DATA_DIR: dataDirectory,
      SABICHEIRO_QA_READY_FILE: readyFile,
    },
    stdio: "ignore",
    windowsHide: false,
  });
  let exit = null;
  child.once("exit", (code, signal) => {
    exit = { code, signal };
  });
  try {
    while (!fs.existsSync(readyFile)) {
      if (exit) throw new Error(`Axvaris exited before ready: ${JSON.stringify(exit)}`);
      const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
      if (elapsedMs > timeoutMs) throw new Error(`Axvaris was not ready after ${timeoutMs} ms`);
      await delay(5);
    }
    return Number(process.hrtime.bigint() - started) / 1_000_000;
  } finally {
    if (!exit) {
      child.kill();
      await Promise.race([
        new Promise((resolve) => child.once("exit", resolve)),
        delay(2_000),
      ]);
    }
  }
}

function summarize(samples) {
  return {
    runs: samples.length,
    p50Ms: Number(percentile(samples, 0.50).toFixed(3)),
    p95Ms: Number(percentile(samples, 0.95).toFixed(3)),
    minMs: Number(Math.min(...samples).toFixed(3)),
    maxMs: Number(Math.max(...samples).toFixed(3)),
    samplesMs: samples.map((value) => Number(value.toFixed(3))),
  };
}

fs.rmSync(probeRoot, { recursive: true, force: true });
fs.mkdirSync(probeRoot, { recursive: true });
try {
  const coldProxy = [];
  for (let index = 0; index < iterations; index += 1) {
    const directory = path.join(probeRoot, `cold-${index}`);
    fs.mkdirSync(directory, { recursive: true });
    const copiedExecutable = path.join(directory, "Axvaris.exe");
    fs.copyFileSync(executable, copiedExecutable);
    coldProxy.push(
      await launchOnce(copiedExecutable, path.join(directory, "data"), path.join(directory, "ready.json")),
    );
  }

  const warmDirectory = path.join(probeRoot, "warm");
  const warm = [];
  for (let index = 0; index < iterations; index += 1) {
    warm.push(
      await launchOnce(executable, warmDirectory, path.join(warmDirectory, "ready.json")),
    );
  }

  const report = {
    generatedAt: new Date().toISOString(),
    executable: path.relative(repository, executable).replaceAll("\\", "/"),
    executableBytes: fs.statSync(executable).size,
    iterations,
    method:
      "process start to the second requestAnimationFrame reported by the production WebView; cold proxy uses unique executable and empty data paths without claiming an OS cache flush",
    coldProxy: summarize(coldProxy),
    warm: summarize(warm),
  };
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
} finally {
  fs.rmSync(probeRoot, { recursive: true, force: true });
}
