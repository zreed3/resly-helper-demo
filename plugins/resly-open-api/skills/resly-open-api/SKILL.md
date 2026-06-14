---
name: resly-open-api
description: Use the installed `resly` CLI to work with the Resly Open API from Codex. Trigger when the user asks for Resly API diagnostics, reservations, room types, rooms, rate plans, inventory, rates and restrictions, webhooks, partner onboarding checks, support troubleshooting, or safe write previews.
---

# Resly Open API

Use the installed `resly` command as the operating surface. Prefer `--json` for anything Codex needs to parse.

## Start

Run these first:

```bash
command -v resly
resly --json doctor
```

Interpret `doctor` before continuing:

- `source: "fixture"` means the CLI is using embedded demo data because credentials are missing or `--fixture` was passed.
- `source: "live"` means credentials/config are available and API calls may reach Resly.
- Auth comes from `RESLY_ACCOUNT_ID` and `RESLY_API_KEY`, then `~/.resly/config.toml`, then explicit flags.

## Safe Reads

Use discovery commands before exact-object reads:

```bash
resly --json account get
resly --json room-types list
resly --json rooms list
resly --json rate-plans list
resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 20
resly --json inventory get --room-type 1BR-GARDEN --from 2026-07-01 --to 2026-07-07
resly --json rates get --rate-plan BAR-1BR-GARDEN --from 2026-07-01 --to 2026-07-07
```

Use exact IDs once discovered:

```bash
resly --json reservations get RSV-1001
resly --json room-types get 1BR-GARDEN
resly --json rooms get 101
```

## Write Previews

Preview rate changes before any live write:

```bash
resly --json rates preview --rate-plan BAR-1BR-GARDEN --file examples/rate-update.json
```

Preview webhook setup:

```bash
resly --json webhooks create --type reservations --url https://example.com/resly/webhook --dry-run
```

## Approval Rules

- Do not run live `rates update`, `webhooks create --live`, `webhooks update --live`, or `webhooks delete --live` unless the user explicitly asks for that exact live write.
- Do not perform production writes unless the user provides the account ID, sets `RESLY_ALLOW_PRODUCTION_WRITES=1`, and explicitly approves the command with `--confirm-account`, `--confirm-environment production`, `--approval-id`, and `--approval-token`.
- Use the approval-token flow for any live write: preview first, have the human approve the exact `METHOD /endpoint`, then apply with `--approval-id` and `--approval-token`.
- Prefer the test API for any live write demonstration.
- Never print or store full API keys, Bearer tokens, guest contact details, or webhook basic-auth passwords in user-facing summaries.
- Never suggest or invent booking delete, bulk reservation mutation, or raw non-GET commands.

## Raw Escape Hatch

Use raw reads only when no high-level command exists:

```bash
resly --json request get /property
resly --json request get "/room-types/1BR-GARDEN/inventory?startDate=2026-07-01&endDate=2026-07-07"
```

Do not use raw non-GET calls for live systems.
