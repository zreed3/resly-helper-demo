import { spawn } from "node:child_process";
import { existsSync, mkdtempSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

const port = Number(process.env.PORT || 8765);
const cliPath = process.env.RESLY_CLI || defaultCliPath();
const tempHome = mkdtempSync(resolve(tmpdir(), "resly-postman-"));

function defaultCliPath() {
  const debugBinary = resolve(process.cwd(), "target/debug/resly");
  return existsSync(debugBinary) ? debugBinary : "resly";
}

function baseEnv(extra = {}) {
  const env = { ...process.env, HOME: tempHome, ...extra };
  for (const key of [
    "RESLY_ACCOUNT_ID",
    "RESLY_API_KEY",
    "RESLY_ACCESS_TOKEN",
    "RESLY_TOKEN",
    "RESLY_ALLOW_PRODUCTION_WRITES"
  ]) {
    if (!(key in extra)) delete env[key];
  }
  return env;
}

function parseJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return { ok: false, raw: text };
  }
}

function runCli(args, { liveEnv = false } = {}) {
  return new Promise((resolveRun) => {
    const child = spawn(cliPath, args, {
      cwd: process.cwd(),
      env: liveEnv
        ? baseEnv({
            RESLY_ACCOUNT_ID: "postman-demo-account",
            RESLY_API_KEY: "postman-demo-key",
            RESLY_BASE_URL: "https://test.api.resly.com.au"
          })
        : baseEnv(),
      stdio: ["ignore", "pipe", "pipe"]
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", (error) => {
      resolveRun({ code: 1, body: { ok: false, error: { code: "spawn.failed", message: error.message } } });
    });
    child.on("close", (code) => {
      const body = parseJson(stdout || stderr);
      resolveRun({ code, body });
    });
  });
}

function sendJson(response, status, body) {
  response.writeHead(status, { "content-type": "application/json; charset=utf-8" });
  response.end(JSON.stringify(body, null, 2));
}

function query(url, key, fallback) {
  return url.searchParams.get(key) || fallback;
}

const server = createServer(async (request, response) => {
  const url = new URL(request.url, `http://${request.headers.host}`);
  if (request.method !== "GET") {
    sendJson(response, 405, { ok: false, error: { code: "method.unsupported" } });
    return;
  }

  if (url.pathname === "/health") {
    sendJson(response, 200, { ok: true, cliPath });
    return;
  }

  if (url.pathname === "/doctor") {
    const result = await runCli(["--json", "--fixture", "doctor"]);
    sendJson(response, result.code === 0 ? 200 : 500, result.body);
    return;
  }

  if (url.pathname === "/availability/quote") {
    const result = await runCli([
      "--json",
      "--fixture",
      "availability",
      "quote",
      "--guests",
      query(url, "guests", "3"),
      "--from",
      query(url, "from", "2026-07-05"),
      "--to",
      query(url, "to", "2026-07-07"),
      "--limit",
      query(url, "limit", "5")
    ]);
    sendJson(response, result.code === 0 ? 200 : 500, result.body);
    return;
  }

  if (url.pathname === "/inventory") {
    const result = await runCli([
      "--json",
      "--fixture",
      "inventory",
      "get",
      "--room-type",
      query(url, "roomType", "2BR-OCEAN"),
      "--from",
      query(url, "from", "2026-07-05"),
      "--to",
      query(url, "to", "2026-07-07")
    ]);
    sendJson(response, result.code === 0 ? 200 : 500, result.body);
    return;
  }

  if (url.pathname === "/rates") {
    const result = await runCli([
      "--json",
      "--fixture",
      "rates",
      "get",
      "--rate-plan",
      query(url, "ratePlan", "BAR-2BR-OCEAN"),
      "--from",
      query(url, "from", "2026-07-05"),
      "--to",
      query(url, "to", "2026-07-07")
    ]);
    sendJson(response, result.code === 0 ? 200 : 500, result.body);
    return;
  }

  if (url.pathname === "/safety/rates-live-without-approval") {
    const result = await runCli(
      [
        "--json",
        "rates",
        "update",
        "--rate-plan",
        "BAR-1BR-GARDEN",
        "--file",
        "examples/rate-update.json",
        "--live",
        "--test-only"
      ],
      { liveEnv: true }
    );
    sendJson(response, result.code === 0 ? 200 : 409, result.body);
    return;
  }

  if (url.pathname === "/safety/webhook-delete-without-confirm") {
    const result = await runCli(
      ["--json", "webhooks", "delete", "resly-reservations", "--live"],
      { liveEnv: true }
    );
    sendJson(response, result.code === 0 ? 200 : 409, result.body);
    return;
  }

  sendJson(response, 404, { ok: false, error: { code: "route.missing", path: url.pathname } });
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Resly Postman wrapper listening on http://127.0.0.1:${port}`);
});

process.on("SIGTERM", () => server.close(() => process.exit(0)));
process.on("SIGINT", () => server.close(() => process.exit(0)));
