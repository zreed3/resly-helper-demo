# RESLY API CLI Demo Plan

Date: 2026-06-13

## Goal

Build a polished `resly` command-line demo that proves Resly's Open API can become a practical operator/developer tool for support, onboarding, integration testing, and partner workflows.

The first version should feel sellable, not experimental: easy auth, stable JSON, clear help, read-only safety by default, and one carefully controlled write workflow.

## Source Notes

Public Resly docs show:

- Product surface: PMS, Channel Manager, commission-free Booking Engine, reporting, direct bookings, trust accounting, unified inbox, and integrations.
- API base URLs:
  - Production: `https://api.resly.com.au`
  - Test: `https://test.api.resly.com.au`
- Auth flow:
  - `POST /token`
  - Body includes `accountId` and API `key`
  - Response returns a Bearer token, valid for 24 hours.
- Main Open API resources:
  - Account/property info
  - Agents
  - Reservations and in-house reservations
  - Blocks
  - Room types/listings
  - Rooms
  - Inventory
  - Rate plans
  - Rates and restrictions
  - Conversations, threads, and messages
  - Webhooks
  - Resly Direct/Express availability and booking flows

Sources:

- https://docs.resly.com.au/docs/authentication-1
- https://docs.resly.com.au/llms.txt
- https://docs.resly.com.au/docs/keeping-reservations-updated-with-resly
- https://docs.resly.com.au/docs/pricing-management-software-integration-guide
- https://www.resly.com.au/

## Positioning

Pitch the CLI as:

> A partner and support toolkit for Resly's Open API: faster onboarding, safer testing, cleaner debugging, and scriptable operations without writing custom code for every integration.

This is not just a developer wrapper. It is a concrete enablement asset Resly could use for:

- Certified partner onboarding
- Support diagnostics
- Channel/integration testing
- API demos during sales calls
- Internal QA of API behavior
- Customer success troubleshooting
- Future AI-agent workflows over Resly data

## Demo Story

The demo should show a believable hospitality/PMS workflow:

1. Configure test credentials.
2. Run `doctor` to verify auth, endpoint reachability, and token refresh.
3. Inspect the account/property.
4. List room types, rooms, rate plans, and agents.
5. Pull upcoming reservations using bounded date filters.
6. Retrieve availability/inventory for a room type.
7. Retrieve rates/restrictions for a rate plan.
8. Preview a rate/restriction update from a local JSON file.
9. Optionally apply the update against the test environment only.
10. Create/list/update/delete a webhook in test mode.
11. Export results as JSON or CSV for handoff.

## Recommended Stack

Use Rust for the first durable demo:

- `clap` for command parsing/help
- `reqwest` for HTTP
- `serde`/`serde_json` for JSON
- `toml` for local config
- `anyhow` or `thiserror` for CLI-friendly errors
- `chrono` for date validation

Why Rust:

- Single fast binary.
- Easy to install into `~/.local/bin`.
- Strong argument validation.
- Great for a tool Resly could ship to partners later.

Fallback if speed matters more than polish: TypeScript/Node with `commander`, native `fetch`, and a `package.json` `bin`.

## Auth And Config

Support credentials in this order:

1. Environment variables:
   - `RESLY_ACCOUNT_ID`
   - `RESLY_API_KEY`
   - `RESLY_BASE_URL`
2. Config file:
   - `~/.resly/config.toml`
3. One-off flags:
   - `--account-id`
   - `--api-key`
   - `--base-url`

Default to the test API unless the user explicitly selects production:

```bash
resly init --env test
resly init --env production
```

Never print full API keys or Bearer tokens. `doctor --json` should report whether auth exists and where it came from, but redact secrets.

## Command Contract

All commands should support human-readable output by default and stable JSON with `--json`.

Core:

```bash
resly --help
resly --json doctor
resly init --account-id <id> --api-key <key> --env test
resly --json token refresh
```

Discovery/read:

```bash
resly --json account get
resly --json agents list
resly --json room-types list
resly --json room-types get <room-type-id>
resly --json rooms list
resly --json rooms get <room-id>
resly --json rate-plans list
resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 50
resly --json reservations get <reservation-id>
resly --json reservations in-house
resly --json blocks list --date-type inBetween --from 2026-07-01 --to 2026-07-31
```

Inventory and pricing:

```bash
resly --json inventory get --room-type <room-type-id> --from 2026-07-01 --to 2026-07-31
resly --json rates get --rate-plan <rate-plan-id> --from 2026-07-01 --to 2026-07-31
resly --json rates preview --rate-plan <rate-plan-id> --file ./rate-update.json
resly --json rates update --rate-plan <rate-plan-id> --file ./rate-update.json --test-only
```

Conversations/messages:

```bash
resly --json conversations list --limit 20
resly --json conversations get <conversation-id>
resly --json threads list --reservation <reservation-id>
resly --json threads get <thread-id>
resly --json messages send --type email --reservation <reservation-id> --subject "Arrival details" --message-file ./message.txt --dry-run
```

Webhooks:

```bash
resly --json webhooks list
resly --json webhooks get <webhook-id>
resly --json webhooks create --type reservation --url https://example.com/resly/webhook --dry-run
resly --json webhooks update <webhook-id> --url https://example.com/resly/webhook --dry-run
resly --json webhooks delete <webhook-id> --dry-run
```

Raw escape hatch:

```bash
resly --json request get /property
resly --json request get "/room-types/{id}/inventory?startDate=2026-07-01&endDate=2026-07-31"
```

Raw non-GET requests should require an explicit `--i-understand-this-is-live` flag.

## MVP Scope

Build the first sellable demo around read-only confidence and one safe write.

MVP commands:

- `doctor`
- `init`
- `token refresh`
- `account get`
- `agents list`
- `room-types list/get`
- `rooms list/get`
- `rate-plans list`
- `reservations list/get/in-house`
- `blocks list`
- `inventory get`
- `rates get`
- `rates preview`
- `webhooks list/create --dry-run`
- `request get`

Defer:

- Live reservation updates
- Live room updates
- Live message sending
- Live webhook delete
- Resly Express booking delivery
- Bulk sync daemon
- Local database/cache
- TUI dashboard

## Safety Rules

- Default to test environment.
- Make all writes dry-run or preview first.
- Require `--live` and environment confirmation for production writes.
- For production, require the exact account ID to be repeated:

```bash
resly rates update --rate-plan <id> --file ./rates.json --live --confirm-account <account-id>
```

- Redact tokens, keys, guest emails, phone numbers, and addresses in logs unless `--include-pii` is explicitly provided.
- Keep `--json` clean: JSON only to stdout, diagnostics to stderr.

## JSON Policy

Use a thin CLI envelope so downstream scripts and AI agents can parse consistently:

```json
{
  "ok": true,
  "command": "reservations.list",
  "environment": "test",
  "data": [],
  "pagination": {
    "limit": 50,
    "nextCursor": null
  }
}
```

Error shape:

```json
{
  "ok": false,
  "command": "reservations.list",
  "error": {
    "code": "auth.missing",
    "message": "Missing RESLY_ACCOUNT_ID or RESLY_API_KEY",
    "retryable": false
  }
}
```

## Implementation Phases

### Phase 1: API Inventory And Contract

- Pull the Resly Markdown docs and OpenAPI references from `llms.txt`.
- Create endpoint notes for each MVP resource.
- Identify query params, path params, date formats, pagination behavior, required scopes, and sample response fields.
- Confirm whether docs have a downloadable OpenAPI spec.
- Decide exact JSON envelope and error codes.

Deliverable: `docs/api-inventory.md` and `docs/command-contract.md`.

### Phase 2: CLI Scaffold

- Create Rust project.
- Add `clap` command tree.
- Add config loader.
- Add token retrieval and cache.
- Add HTTP client with auth refresh on 401.
- Add redaction helpers.
- Add `doctor`, `init`, and `request get`.

Deliverable: installable `resly` binary with auth diagnostics.

### Phase 3: Read-Only Demo Commands

- Implement account/property, agents, room types, rooms, rate plans.
- Implement reservations list/get/in-house.
- Implement blocks, inventory, and rates reads.
- Add bounded date validation.
- Add fixtures for tests without live credentials.

Deliverable: read-only demo flow works with `--json`.

### Phase 4: Safe Write Demo

- Implement `rates preview`.
- Implement `webhooks create --dry-run`.
- Optionally implement `rates update --test-only`.
- Require explicit flags for live writes.
- Add request body tests to prevent malformed updates.

Deliverable: one controlled write path that proves operational value.

### Phase 5: Packaging And Sales Demo

- Add README with quickstart and demo script.
- Add `make install-local`.
- Smoke test from `/tmp`:

```bash
command -v resly
resly --help
resly --json doctor
```

- Record a 2-3 minute terminal demo.
- Create a one-page sell-back brief for Resly.

Deliverable: runnable CLI, demo video, and sales brief.

## Suggested Repo Structure

```text
.
├── README.md
├── RESLY_CLI_DEMO_PLAN.md
├── docs/
│   ├── api-inventory.md
│   ├── command-contract.md
│   └── sell-back-brief.md
├── fixtures/
│   ├── account.json
│   ├── reservations.json
│   └── rates-and-restrictions.json
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── auth.rs
│   ├── client.rs
│   ├── output.rs
│   └── commands/
└── Makefile
```

## Demo Script

```bash
resly --json doctor
resly --json account get
resly --json room-types list
resly --json rate-plans list
resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 10
resly --json inventory get --room-type 1B-GARDEN --from 2026-07-01 --to 2026-07-07
resly --json rates get --rate-plan 3N-1B-GARDEN --from 2026-07-01 --to 2026-07-07
resly --json rates preview --rate-plan 3N-1B-GARDEN --file ./examples/rate-update.json
resly --json webhooks create --type reservation --url https://example.com/resly/webhook --dry-run
```

## Sell-Back Brief Outline

Title:

> Resly CLI: a partner onboarding and API operations toolkit

Core value:

- Reduces partner onboarding friction.
- Gives Resly support a repeatable diagnostic tool.
- Makes API demos tangible.
- Encourages safer test-environment usage.
- Creates a foundation for SDKs, internal tools, and AI-assisted support workflows.

Possible commercial paths:

- Fixed-fee prototype handoff.
- Paid pilot with Resly support/engineering.
- Ongoing maintenance retainer.
- Partner toolkit package: CLI + docs + demo fixtures + CI smoke tests.

Suggested offer:

- Prototype: 1-2 weeks.
- Pilot hardening: 2-4 weeks.
- Optional maintenance: monthly retainer for API changes and partner requests.

## Open Questions

- Can Resly provide test credentials and a sandbox account with realistic data?
- Is there an official downloadable OpenAPI spec, or only Markdown/ReadMe pages?
- What pagination style do list endpoints use in practice?
- Which write action is safest and most impressive for the first live demo?
- Do they prefer a public partner CLI, an internal support CLI, or both?
- Should the CLI support CSV exports for support/customer success users?
- Should PII redaction be enabled by default for support workflows?

## Success Criteria

- Installs with one command.
- Works outside the source repo.
- `doctor --json` clearly diagnoses setup/auth.
- All MVP read commands return stable JSON.
- Date-based commands validate input before hitting the API.
- No command leaks credentials.
- Writes require dry-run/preview before live execution.
- Demo can be run in under five minutes.
- Sales brief clearly maps the tool to Resly business value.
