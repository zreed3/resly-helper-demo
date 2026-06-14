# Human Usage Guide

This guide is for someone demoing or evaluating the Resly CLI package.

## Install

```bash
make install-local
command -v resly
```

The install target builds the Rust release binary and copies it to `~/.local/bin/resly`.

## Run Without Credentials

The CLI includes fixture data, so you can run the demo immediately:

```bash
resly --json doctor
resly --json account get
resly --json agents list
resly --json room-types list
resly --json rooms list
resly --json rate-plans list
```

`doctor` reports `fixtureMode: true` when no Resly credentials are configured.

## Availability Demo

Use this when a manager asks a normal guest question from the desk or their phone:

```bash
resly --json availability quote --guests 3 --from 2026-07-05 --to 2026-07-07
```

The CLI checks room types, matching rate plans, inventory, stop-sells, and nightly rates. It returns the cheapest available option first, plus other options that fit the guest count.

## Configure Resly Test Access

```bash
resly init --account-id YOUR_ACCOUNT_ID --api-key YOUR_API_KEY --env test
resly --json doctor
```

This writes `~/.resly/config.toml`.

Environment variables override the config file:

```bash
RESLY_ACCOUNT_ID=YOUR_ACCOUNT_ID \
RESLY_API_KEY=YOUR_API_KEY \
RESLY_BASE_URL=https://test.api.resly.com.au \
resly --json doctor
```

## Reservation Demo

```bash
resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 10
resly --json reservations get RSV-1001
resly --json reservations in-house
```

Use `--date-type updated --start-time ... --end-time ...` for sync-style workflows.

## Pricing Demo

```bash
resly --json rate-plans list
resly --json rates get --rate-plan BAR-1BR-GARDEN --from 2026-07-01 --to 2026-07-07
resly --json rates preview --rate-plan BAR-1BR-GARDEN --file examples/rate-update.json
```

Preview creates a short-lived approval request. A human approves the exact method and endpoint, then live test-environment rate updates require `--live`, `--test-only`, the approval ID, and the one-time approval token:

```bash
resly --json rates preview --rate-plan BAR-1BR-GARDEN --file examples/rate-update.json
resly --json approvals approve apr_123 \
  --confirm-operation 'PATCH /rate-plans/BAR-1BR-GARDEN/rates-and-restrictions'
resly --json rates update \
  --rate-plan BAR-1BR-GARDEN \
  --file examples/rate-update.json \
  --live \
  --test-only \
  --approval-id apr_123 \
  --approval-token rat_...
```

Production writes require the same approval flow plus explicit account/environment confirmation and an opt-in environment variable:

```bash
RESLY_ALLOW_PRODUCTION_WRITES=1 \
resly --json rates update \
  --rate-plan BAR-1BR-GARDEN \
  --file examples/rate-update.json \
  --live \
  --approval-id apr_123 \
  --approval-token rat_... \
  --confirm-account YOUR_ACCOUNT_ID \
  --confirm-environment production
```

## Webhook Demo

```bash
resly --json webhooks list
resly --json webhooks create --type reservations --url https://example.com/resly/webhook --dry-run
```

Without `--live`, webhook create/update/delete commands only preview the request.
Live webhook writes use the same `resly approvals approve` and `--approval-id`/`--approval-token` flow as rate updates. Live webhook deletes also require `--confirm-delete <webhook-id>`.

## Safety Review

Read the full guardrail plan at:

```text
docs/SAFETY_REVIEW_AND_STRATEGY.md
```

The CLI now implements the core two-phase approval flow: preview creates an approval ID, a human approves the exact operation, and live apply is refused if the payload, account, environment, method, or endpoint changes.

## Codex Plugin

The Codex plugin lives at:

```text
plugins/resly-open-api
```

It contains a skill that tells Codex to start with:

```bash
command -v resly
resly --json doctor
```

Then it guides Codex through safe reads, previews, and explicit-only live writes.

## Claude Plugin

The Claude integration lives at:

```text
claude/resly-mcp
```

Run:

```bash
cd claude/resly-mcp
npm install
npm run smoke
```

For local Claude Desktop configuration, start from:

```text
claude/resly-mcp/claude_desktop_config.example.json
```

For one-click extension distribution, use:

```text
claude/resly-mcp/resly-open-api.mcpb
```

## Troubleshooting

- `resly --json doctor` says fixture mode: credentials are missing. That is fine for demos.
- `auth.failed`: confirm the account ID/API key pair and base URL.
- `api.error`: check the API scope for the endpoint.
- `date.invalid`: use `YYYY-MM-DD`.
- Claude MCP cannot find `resly`: set `RESLY_CLI` to the absolute path from `command -v resly`.
