# Safety Review And Guardrail Strategy

Date: 2026-06-14

## Executive Summary

Yes, we can build hard guardrails around Claude, Codex, and the Resly CLI so agents cannot mutate bookings, rates, rooms, or webhooks unless a human explicitly approves the exact change.

The main rule is simple:

> Agents may discover, analyze, and preview. Humans approve. Deterministic code enforces the boundary.

Do not rely on prompt instructions alone. Codex and Claude both provide approval and permission concepts, but the Resly integration should enforce safety in its own CLI and MCP server so a mistaken or over-eager agent cannot bypass the business rule.

## Current Safety Posture

Existing controls already present:

- Fixture mode is automatic when credentials are missing.
- The CLI defaults write-like commands to preview behavior.
- Raw escape hatch is GET-only.
- Claude MCP server exposes read and preview tools only.
- Codex plugin skill tells agents not to perform live writes without explicit approval.
- Live rate and webhook writes require a short-lived approval ID and approval token.
- Approval checks are bound to command, method, endpoint, account, environment, base URL, and payload hash.
- Production writes require `RESLY_ALLOW_PRODUCTION_WRITES=1`, `--confirm-account`, and `--confirm-environment production`.
- Test-only rate updates require `--test-only`.

Remaining gaps:

- There is no append-only local audit log of proposed and applied changes.
- There is no shared policy file declaring which operations are allowed for agents.
- There is no hard distinction between an agent-facing CLI profile and a human/operator-facing CLI profile.
- Future destructive classes should keep the same pattern as webhook delete: typed resource confirmation plus an approval token.

## Non-Negotiable Safety Principles

1. Agents never get generic mutation tools.
2. Agents never get booking delete/cancel tools in the first sellable version.
3. Every write is two-phase: preview first, apply second.
4. Approval must be bound to the exact operation, account, environment, resource, and payload hash.
5. Production writes require extra friction.
6. Destructive actions require typed resource confirmation.
7. API keys used by agents should be read-only where Resly scopes allow it.
8. All write attempts should be auditable, including denied attempts.
9. Raw requests remain read-only.
10. The safest MCP/Codex plugin is one that simply cannot call live writes.

## Recommended Guardrail Architecture

```text
Claude / Codex
   |
   | read + preview tools only
   v
Resly MCP / Codex Plugin
   |
   | calls installed CLI with safe command set
   v
resly CLI
   |
   | operation registry + approval token + policy checks
   v
Resly API
```

## Operation Classes

Every command should be registered with a safety class:

| Class | Examples | Agent exposure | Human approval |
| --- | --- | --- | --- |
| `read` | `reservations list`, `rates get`, `inventory get` | Allowed | Not required |
| `preview` | `rates preview`, `webhooks create --dry-run` | Allowed | Not required |
| `write-low` | Create test webhook | Hidden by default | Approval token |
| `write-medium` | Update rates/restrictions | Hidden from MCP | Approval token + environment confirmation |
| `destructive` | Delete webhook, cancel/delete booking | Not exposed | Typed confirmation + approval token + elevated policy |
| `forbidden` | Delete all bookings, raw POST/DELETE | Not implemented | Not allowed |

## Two-Phase Approval Design

### Phase 1: Preview

The agent or human runs:

```bash
resly --json rates preview --rate-plan BAR-1BR-GARDEN --file examples/rate-update.json
```

The CLI should return:

```json
{
  "ok": true,
  "command": "rates.preview",
  "data": {
    "live": false,
    "operation": "rates.update",
    "environment": "test",
    "accountId": "demo-account",
    "resource": "BAR-1BR-GARDEN",
    "approval": {
      "id": "apr_...",
      "operation": "PATCH /rate-plans/BAR-1BR-GARDEN/rates-and-restrictions",
      "payloadHash": "...",
      "expiresAt": 1781439300,
      "approveCommand": "resly approvals approve apr_... --confirm-operation 'PATCH /rate-plans/BAR-1BR-GARDEN/rates-and-restrictions'"
    },
    "summary": {
      "datesTouched": 2,
      "minDate": "2026-07-04",
      "maxDate": "2026-07-05",
      "maxRateIncreasePercent": 11.2
    }
  }
}
```

### Phase 2: Explicit Human Approval

The human approves the exact preview:

```bash
resly approvals approve apr_123 \
  --confirm-operation 'PATCH /rate-plans/BAR-1BR-GARDEN/rates-and-restrictions'
```

The approval command writes only a token hash to the local approval store and prints the one-time approval token.

### Phase 3: Apply

Only then can a live write run:

```bash
resly --json rates update \
  --rate-plan BAR-1BR-GARDEN \
  --file examples/rate-update.json \
  --live \
  --test-only \
  --approval-id apr_123 \
  --approval-token rat_...
```

The CLI recomputes the payload hash. If the file changed after approval, the write is refused.

## Production Write Policy

Production writes should require all of:

```bash
--live
--approval-id apr_...
--approval-token rat_...
--confirm-account <account-id>
--confirm-environment production
```

Additionally, production write support should be disabled unless:

```bash
RESLY_ALLOW_PRODUCTION_WRITES=1
```

This prevents accidental production writes from a copied command.

## Booking Safety

For bookings/reservations:

- Do not implement delete booking.
- Do not implement cancel booking in the agent-facing MCP server.
- If update reservation is added later, start with `reservations preview-update` only.
- Require approval tokens for any reservation mutation.
- Require a before/after diff and guest-impact summary.
- Require typed confirmation of the reservation ID.
- Block bulk reservation mutation until a separate safety review is complete.

Example blocked command shape:

```bash
resly reservations delete --all
```

Expected result:

```json
{
  "ok": false,
  "error": {
    "code": "operation.forbidden",
    "message": "Bulk reservation deletion is not implemented by this CLI."
  }
}
```

## Claude Guardrails

Current Claude MCP server should remain preview-only.

Recommended MCP server policy:

- Expose `doctor`, reads, and preview tools.
- Do not expose `rates update`, webhook live write, room update, or reservation mutation tools.
- If write approval is needed, expose only `create_approval_request`; never expose `apply_live_change`.
- Use Claude tool approval prompts as an additional layer, not the primary safety control.
- Keep API credentials scoped to read-only for the Claude connector whenever Resly scopes allow it.

## Codex Guardrails

Recommended Codex setup:

- Keep the Codex skill instruction: discover, read, preview, stop.
- Add a project-level safety policy file, for example `safety/resly-policy.toml`.
- Use Codex sandbox/approval settings for local command execution.
- Do not rely on Codex approval prompts alone for API writes.
- Make live write commands require approval tokens so even an approved shell command cannot mutate the wrong resource unless the exact preview was approved.

## Proposed Policy File

```toml
[defaults]
mode = "preview"
allow_live_writes = false
allow_production_writes = false
allow_bulk_mutations = false

[agents.claude]
allowed_classes = ["read", "preview"]

[agents.codex]
allowed_classes = ["read", "preview"]

[humans.operator]
allowed_classes = ["read", "preview", "write-low", "write-medium"]
requires_approval_token = true

[forbidden]
operations = [
  "reservations.delete",
  "reservations.bulk-update",
  "raw.post",
  "raw.patch",
  "raw.delete"
]
```

## Audit Log

Create append-only JSONL logs under:

```text
~/.resly/audit/YYYY-MM-DD.jsonl
```

Log events:

- Preview created.
- Approval granted.
- Approval expired.
- Write attempted.
- Write applied.
- Write denied.

Each event should include:

- Timestamp.
- Command.
- Actor source: `human-cli`, `codex`, `claude-mcp`.
- Environment.
- Account ID redacted.
- Resource ID.
- Payload hash.
- Safety class.
- Outcome.

Never log full API keys, Bearer tokens, guest contact details, or webhook passwords.

## Rate Update Guardrails

Rate updates should have configurable limits:

- Maximum dates touched per approval.
- Maximum percentage increase/decrease.
- No same-day update unless `--confirm-same-day`.
- No stop-sell changes unless explicitly confirmed.
- No production write without `RESLY_ALLOW_PRODUCTION_WRITES=1`.
- Refuse payloads with missing `echoToken`.
- Refuse payloads whose post-approval hash changed.

## Webhook Guardrails

Webhook create/update/delete should require:

- Preview first.
- Valid HTTPS URL unless explicitly in local dev mode.
- Redaction of basic-auth password in output.
- For delete: `--confirm-delete <webhook-id>`.
- Approval token for live update/delete.

## Implementation Plan

Status: phases 1, 2, and the core phase 3 enforcement are implemented in this demo. Audit logging, a shared policy file, and deeper automated tests remain recommended follow-up work.

### Phase 1: Policy And Documentation

- Add this safety strategy.
- Add safety page to marketing site.
- Update CLI README to describe hard guardrails.
- Update Codex skill and Claude README to say live writes are intentionally not exposed.

### Phase 2: CLI Approval Store

- `resly approvals list|show|approve|revoke`.
- Store pending approvals in `~/.resly/approvals/`.
- Bind approvals to payload hash, account, environment, base URL, method, and endpoint.
- Add expiry.
- Add audit log. Not yet implemented.

### Phase 3: Enforce Approval Tokens

- Require approval tokens for:
  - `rates update --live`
  - `webhooks create --live`
  - `webhooks update --live`
  - `webhooks delete --live`
- Require `--confirm-delete <id>` for deletes.
- Require `--confirm-environment production` for production.
- Gate production writes behind `RESLY_ALLOW_PRODUCTION_WRITES=1`.

### Phase 4: Agent Surface Hardening

- Keep Claude MCP preview-only.
- Add optional `create_approval_request` tool only if useful.
- Do not add MCP `apply` tools.
- Update Codex skill to require approval workflow before suggesting any live write command.

### Phase 5: Tests

Add tests for:

- Live write without approval is refused.
- Preview creates approval metadata.
- Payload changed after approval is refused.
- Production write without env gate is refused.
- Delete without `--confirm-delete` is refused.
- Claude MCP exposes no live write tool.
- Raw request supports GET only.

## Open Decisions

- Should the demo ever support live writes from Claude/Codex, or should live writes be human CLI only?
- Should production writes be entirely disabled in the public demo build?
- Should Resly provide read-only partner API keys for agent workflows?
- Should approval tokens be local-only or integrate with a shared approval system later?
- Should booking mutations be permanently out of scope for the agent layer?

## Recommended Default

For the sellable demo:

- Claude: read + preview only.
- Codex: read + preview only.
- CLI: live writes possible only with approval token.
- Production writes disabled by default.
- Booking delete/cancel not implemented.

This gives Resly a much stronger story than “the model promises to ask first.” It makes the dangerous path technically unavailable unless the human has approved the exact change.
