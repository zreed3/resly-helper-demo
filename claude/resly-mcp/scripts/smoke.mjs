import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const transport = new StdioClientTransport({
  command: "node",
  args: ["server/index.js"],
  env: {
    ...process.env,
    RESLY_CLI: process.env.RESLY_CLI || "resly"
  }
});

const client = new Client({
  name: "resly-mcp-smoke",
  version: "0.1.0"
});

await client.connect(transport);
const tools = await client.listTools();
if (!tools.tools.some((tool) => tool.name === "doctor")) {
  throw new Error("doctor tool was not registered");
}
const result = await client.callTool({ name: "doctor", arguments: {} });
const text = result.content?.[0]?.text || "";
if (!text.includes('"command": "doctor"')) {
  throw new Error("doctor tool did not return Resly doctor JSON");
}
await client.close();
console.log("Resly MCP smoke passed");
