# Command Contract

## Global Flags

```bash
resly --json <command>
resly --fixture <command>
resly --base-url https://test.api.resly.com.au <command>
resly --account-id <id> --api-key <key> <command>
```

## JSON Envelope

Success:

```json
{
  "ok": true,
  "command": "reservations.list",
  "environment": "test",
  "source": "fixture",
  "data": {}
}
```

Error:

```json
{
  "ok": false,
  "error": {
    "code": "auth.missing",
    "message": "Missing RESLY_ACCOUNT_ID or account_id in ~/.resly/config.toml",
    "retryable": false
  }
}
```

## Core Commands

```bash
resly --json doctor
resly init --account-id <id> --api-key <key> --env test
resly --json token refresh
```

## Discovery And Reads

```bash
resly --json account get
resly --json agents list
resly --json room-types list
resly --json room-types get <room-type-id>
resly --json rooms list
resly --json rooms get <room-id>
resly --json rate-plans list
```

## Reservations

```bash
resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 20
resly --json reservations list --date-type updated --start-time 2026-07-01T00:00:00Z --end-time 2026-07-02T00:00:00Z
resly --json reservations get <reservation-id>
resly --json reservations in-house
```

## Inventory, Blocks, Rates

```bash
resly --json blocks list --date-type in-between --from 2026-07-01 --to 2026-07-31
resly --json inventory get --room-type <room-type-id> --from 2026-07-01 --to 2026-07-07
resly --json rates get --rate-plan <rate-plan-id> --from 2026-07-01 --to 2026-07-07
resly --json rates preview --rate-plan <rate-plan-id> --file examples/rate-update.json
resly --json approvals approve <approval-id> --confirm-operation 'PATCH /rate-plans/<rate-plan-id>/rates-and-restrictions'
resly --json rates update --rate-plan <rate-plan-id> --file examples/rate-update.json --live --test-only --approval-id <approval-id> --approval-token <token>
```

Live writes are refused unless the approval matches the exact command, method, endpoint, account, environment, and payload hash. Production writes also require `RESLY_ALLOW_PRODUCTION_WRITES=1`, `--confirm-account <account-id>`, and `--confirm-environment production`.

## Webhooks

```bash
resly --json webhooks list
resly --json webhooks get <webhook-id>
resly --json webhooks create --type reservations --url https://example.com/resly/webhook --dry-run
resly --json webhooks update <webhook-id> --url https://example.com/resly/webhook --dry-run
resly --json webhooks delete <webhook-id> --dry-run
```

Add `--live --approval-id <approval-id> --approval-token <token>` only after approving the exact dry-run operation. Live webhook deletes also require `--confirm-delete <webhook-id>`. Production webhook writes use the same production confirmations as rate updates.

Supported webhook types:

- `reservations`
- `blocks`
- `rooms`
- `messages`
- `room-types`

## Raw Read Escape Hatch

```bash
resly --json request get /property
resly --json request get "/room-types/1BR-GARDEN/inventory?startDate=2026-07-01&endDate=2026-07-07"
```

Only GET is exposed through the raw command.
