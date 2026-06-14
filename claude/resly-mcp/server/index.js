#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";

const server = new McpServer({
  name: "resly-open-api",
  version: "0.1.0"
});

const DEFAULT_FROM = "2026-07-01";
const DEFAULT_TO = "2026-07-31";

function textResult(value) {
  return {
    content: [
      {
        type: "text",
        text: typeof value === "string" ? value : JSON.stringify(value, null, 2)
      }
    ]
  };
}

function cliCommand() {
  return process.env.RESLY_CLI || "resly";
}

function cliEnv() {
  const env = { ...process.env };
  for (const key of ["RESLY_ACCOUNT_ID", "RESLY_API_KEY", "RESLY_BASE_URL", "RESLY_ACCESS_TOKEN", "RESLY_TOKEN"]) {
    if (process.env[key]) env[key] = process.env[key];
  }
  return env;
}

function runResly(args) {
  return new Promise((resolve, reject) => {
    const child = spawn(cliCommand(), args, {
      env: cliEnv(),
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
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        try {
          resolve(JSON.parse(stdout));
        } catch {
          resolve({ ok: true, text: stdout.trim() });
        }
      } else {
        reject(new Error(stderr.trim() || stdout.trim() || `resly exited with ${code}`));
      }
    });
  });
}

server.registerTool(
  "doctor",
  {
    title: "Check Resly CLI setup",
    description: "Checks whether the Resly CLI is installed, configured, using fixture mode, and able to authenticate.",
    inputSchema: {}
  },
  async () => textResult(await runResly(["--json", "doctor"]))
);

server.registerTool(
  "list_room_types",
  {
    title: "List Resly room types",
    description: "Lists room types/listings from Resly or embedded fixture data.",
    inputSchema: {
      showPhotos: z.boolean().optional().describe("Include photos when supported by the API."),
      portfolioId: z.string().optional().describe("Optional Resly portfolio ID filter.")
    }
  },
  async ({ showPhotos, portfolioId }) => {
    const args = ["--json", "room-types", "list"];
    if (showPhotos) args.push("--show-photos");
    if (portfolioId) args.push("--portfolio-id", portfolioId);
    return textResult(await runResly(args));
  }
);

server.registerTool(
  "list_reservations",
  {
    title: "List Resly reservations",
    description: "Lists reservations for a bounded check-in date range.",
    inputSchema: {
      from: z.string().default(DEFAULT_FROM).describe("Start date in YYYY-MM-DD format."),
      to: z.string().default(DEFAULT_TO).describe("End date in YYYY-MM-DD format."),
      limit: z.number().int().min(1).max(100).default(20),
      status: z.string().default("confirmed"),
      roomId: z.string().optional()
    }
  },
  async ({ from = DEFAULT_FROM, to = DEFAULT_TO, limit = 20, status = "confirmed", roomId }) => {
    const args = [
      "--json",
      "reservations",
      "list",
      "--from",
      from,
      "--to",
      to,
      "--limit",
      String(limit),
      "--status",
      status
    ];
    if (roomId) args.push("--room-id", roomId);
    return textResult(await runResly(args));
  }
);

server.registerTool(
  "get_rates",
  {
    title: "Get Resly rates and restrictions",
    description: "Reads rates and restrictions for a Resly rate plan over a bounded date range.",
    inputSchema: {
      ratePlan: z.string().describe("Resly rate plan ID."),
      from: z.string().default("2026-07-01"),
      to: z.string().default("2026-07-07")
    }
  },
  async ({ ratePlan, from = "2026-07-01", to = "2026-07-07" }) =>
    textResult(await runResly(["--json", "rates", "get", "--rate-plan", ratePlan, "--from", from, "--to", to]))
);

server.registerTool(
  "quote_availability",
  {
    title: "Quote Resly availability",
    description: "Answers a manager-friendly availability question by guest count and stay dates.",
    inputSchema: {
      guests: z.number().int().min(1).describe("Number of guests to fit."),
      from: z.string().default("2026-07-05").describe("Check-in date in YYYY-MM-DD format."),
      to: z.string().default("2026-07-07").describe("Check-out date in YYYY-MM-DD format."),
      limit: z.number().int().min(1).max(20).default(5)
    }
  },
  async ({ guests, from = "2026-07-05", to = "2026-07-07", limit = 5 }) =>
    textResult(
      await runResly([
        "--json",
        "availability",
        "quote",
        "--guests",
        String(guests),
        "--from",
        from,
        "--to",
        to,
        "--limit",
        String(limit)
      ])
    )
);

server.registerTool(
  "preview_rate_update",
  {
    title: "Preview Resly rate update",
    description: "Validates and previews a rate/restriction update JSON file without sending a live write.",
    inputSchema: {
      ratePlan: z.string().describe("Resly rate plan ID."),
      file: z.string().describe("Path to a local JSON payload with echoToken and restrictions[].")
    }
  },
  async ({ ratePlan, file }) =>
    textResult(await runResly(["--json", "rates", "preview", "--rate-plan", ratePlan, "--file", file]))
);

server.registerTool(
  "preview_webhook",
  {
    title: "Preview Resly webhook",
    description: "Previews webhook creation without sending a live write.",
    inputSchema: {
      type: z.enum(["reservations", "blocks", "rooms", "messages", "room-types"]).default("reservations"),
      url: z.string().url()
    }
  },
  async ({ type = "reservations", url }) =>
    textResult(await runResly(["--json", "webhooks", "create", "--type", type, "--url", url, "--dry-run"]))
);

server.registerTool(
  "raw_get",
  {
    title: "Raw Resly GET",
    description: "Read-only escape hatch for Resly API paths not yet covered by high-level tools.",
    inputSchema: {
      path: z.string().describe("API path such as /property or /room-types.")
    }
  },
  async ({ path }) => textResult(await runResly(["--json", "request", "get", path]))
);

server.registerResource(
  "demo-rate-update",
  "resly://examples/rate-update",
  {
    title: "Demo rate update payload",
    description: "Example JSON payload for preview_rate_update.",
    mimeType: "application/json"
  },
  async () => {
    const candidates = [
      process.env.RESLY_RATE_UPDATE_EXAMPLE,
      new URL("../../../examples/rate-update.json", import.meta.url)
    ].filter(Boolean);
    for (const candidate of candidates) {
      try {
        const text = await readFile(candidate, "utf8");
        return { contents: [{ uri: "resly://examples/rate-update", mimeType: "application/json", text }] };
      } catch {
        continue;
      }
    }
    return {
      contents: [
        {
          uri: "resly://examples/rate-update",
          mimeType: "application/json",
          text: JSON.stringify({ echoToken: "demo", restrictions: [{ date: "2026-07-04", rate: 385 }] }, null, 2)
        }
      ]
    };
  }
);

const transport = new StdioServerTransport();
await server.connect(transport);
