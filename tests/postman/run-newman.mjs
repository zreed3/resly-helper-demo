import { spawn } from "node:child_process";
import { setTimeout as wait } from "node:timers/promises";

const port = Number(process.env.PORT || 8765);
const baseUrl = `http://127.0.0.1:${port}`;

function spawnCommand(command, args, options = {}) {
  return spawn(command, args, {
    stdio: options.stdio || "inherit",
    env: { ...process.env, ...options.env }
  });
}

async function waitForHealth() {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    try {
      const response = await fetch(`${baseUrl}/health`);
      if (response.ok) return;
    } catch {
      await wait(250);
    }
  }
  throw new Error(`Postman wrapper did not become healthy at ${baseUrl}`);
}

function runNewman() {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawnCommand(
      "npx",
      [
        "--yes",
        "newman",
        "run",
        "tests/postman/resly-helper.postman_collection.json",
        "--env-var",
        `baseUrl=${baseUrl}`
      ],
      { stdio: "inherit" }
    );
    child.on("error", rejectRun);
    child.on("close", (code) => {
      if (code === 0) resolveRun();
      else rejectRun(new Error(`newman exited with ${code}`));
    });
  });
}

const server = spawnCommand("node", ["tests/postman/server.mjs"], {
  env: { PORT: String(port) },
  stdio: ["ignore", "pipe", "pipe"]
});

server.stdout.on("data", (chunk) => process.stdout.write(chunk));
server.stderr.on("data", (chunk) => process.stderr.write(chunk));

try {
  await waitForHealth();
  await runNewman();
} finally {
  server.kill("SIGTERM");
}
