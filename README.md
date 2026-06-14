# Resly Helper Demo

This repo turns the public Resly Open API into a runnable demo package:

- `resly`: a Rust CLI for Resly API diagnostics, reads, safe previews, and raw GETs.
- Codex plugin: a repo-local plugin with a Resly skill for future Codex agents.
- Claude plugin: a local MCP server and `.mcpb` bundle for Claude Desktop.
- Human docs: setup, command examples, API contract, and sell-back positioning.
- Marketing site: multipage static site under `site/`.

The CLI works without credentials by falling back to embedded fixtures. Add Resly test credentials when you want live API reads.

## License

Public source access is provided under a custom non-commercial source-available license.
Copyright (c) 2026 Otterblock Pty Ltd. All rights reserved.

See [LICENSE](LICENSE). Commercial use, resale, sublicensing, hosted services, and paid implementation work require prior written permission from Otterblock Pty Ltd.

## Quick Start

```bash
make install-local
resly --json doctor
resly --json room-types list
resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 10
resly --json rates preview --rate-plan BAR-1BR-GARDEN --file examples/rate-update.json
```

## Live API Setup

Use Resly's test API for demos:

```bash
resly init --account-id YOUR_ACCOUNT_ID --api-key YOUR_API_KEY --env test
resly --json doctor
resly --json account get
```

You can also use environment variables:

```bash
export RESLY_ACCOUNT_ID=YOUR_ACCOUNT_ID
export RESLY_API_KEY=YOUR_API_KEY
export RESLY_BASE_URL=https://test.api.resly.com.au
```

The CLI retrieves a Bearer token from `/token` and caches it under `~/.resly/token-cache.json`.

## Safety Model

- Missing credentials automatically use fixture mode.
- Reads support stable `--json` envelopes.
- Write-like commands preview by default.
- Live writes require `--live`, `--approval-id`, and `--approval-token`.
- Approval tokens are bound to the exact command, method, endpoint, account, environment, base URL, and payload hash.
- Production writes also require `RESLY_ALLOW_PRODUCTION_WRITES=1`, `--confirm-account`, and `--confirm-environment production`.
- Booking delete/cancel commands are not implemented in the demo.
- Tokens and API keys are redacted in output.

See [Safety review and guardrail strategy](docs/SAFETY_REVIEW_AND_STRATEGY.md) for the hard-approval model: two-phase previews, short-lived approval tokens, production write friction, and preview-only Claude/Codex tool surfaces.

## Included Integrations

Codex plugin:

```text
plugins/resly-open-api/
```

Claude MCP plugin:

```bash
cd claude/resly-mcp
npm install
npm run smoke
```

The packaged Claude Desktop bundle is:

```text
claude/resly-mcp/resly-open-api.mcpb
```

## Documentation

- [Human usage guide](docs/HUMAN_USAGE.md)
- [Command contract](docs/COMMAND_CONTRACT.md)
- [API inventory](docs/API_INVENTORY.md)
- [Sell-back brief](docs/SELL_BACK_BRIEF.md)
- [Safety review and guardrail strategy](docs/SAFETY_REVIEW_AND_STRATEGY.md)
- [Original demo plan](RESLY_CLI_DEMO_PLAN.md)

## Verification

```bash
cargo test
make install-local
cd /tmp && resly --json doctor
cd "/Users/zach/Documents/Resley CLI/claude/resly-mcp" && npm run smoke
```
