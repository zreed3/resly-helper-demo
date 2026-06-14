# Resly Claude MCP Plugin

This package exposes the installed `resly` CLI to Claude through a local MCP stdio server.

## Tools

- `doctor`
- `list_room_types`
- `list_reservations`
- `get_rates`
- `quote_availability`
- `preview_rate_update`
- `preview_webhook`
- `raw_get`

All write-like tools are previews only. Live writes stay out of the Claude MCP tool surface. The CLI requires a short-lived approval token bound to the exact previewed method, endpoint, account, environment, and payload before any live mutation can run.

Do not add live booking mutation, booking deletion, raw POST/PATCH/DELETE, or bulk update tools to this MCP server without a separate safety review.

## Local Development

```bash
npm install
npm run smoke
```

## Claude Desktop Config

Copy `claude_desktop_config.example.json` into your Claude Desktop MCP config and update paths if needed.

## MCPB Bundle

`manifest.json` is bundle-ready for Claude Desktop's `.mcpb` extension format. Package it with the MCPB CLI after dependencies are installed:

```bash
npx @anthropic-ai/mcpb pack .
```
