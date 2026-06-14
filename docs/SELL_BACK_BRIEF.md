# Sell-Back Brief

## Offer

Resly CLI is a partner onboarding and API operations toolkit for Resly's Open API.

It packages the API into:

- A scriptable CLI.
- A Codex plugin for AI-assisted support and engineering workflows.
- A Claude Desktop MCP plugin.
- Fixture data for demos without customer access.
- Human documentation and a marketing site.

## Why Resly Might Care

Resly already has a strong API surface. The CLI makes that surface easier to test, explain, support, and sell.

Practical value:

- Faster certified partner onboarding.
- Repeatable support diagnostics.
- Safer test-environment writes.
- Better sales demos for pricing, inventory, reservations, and webhooks.
- A foundation for official SDKs and AI-agent workflows.

## Demo Narrative

1. Run `resly --json doctor`.
2. Show fixture mode working with no secrets.
3. Configure test credentials.
4. List room types, rooms, rate plans, and reservations.
5. Inspect inventory and rates for a room type/rate plan.
6. Preview a rate update from JSON.
7. Preview webhook creation.
8. Show Codex and Claude using the same safe command layer.

## Commercial Packaging

Prototype handoff:

- CLI source and binary install target.
- Codex plugin.
- Claude MCP bundle.
- Docs and demo fixtures.
- Short implementation walkthrough.

Pilot hardening:

- Test credentials from Resly.
- Endpoint-by-endpoint confirmation.
- CI smoke tests against Resly's test API.
- Partner-ready install docs.
- Optional CSV exports and redaction policy.

Maintenance:

- Monthly API compatibility updates.
- New endpoint coverage.
- Partner workflow additions.
- Claude/Codex plugin updates.

## Suggested Positioning

> Give every partner, support engineer, and AI assistant a safe command layer for Resly's Open API.

## Open Questions For Resly

- Can they provide a sandbox account with realistic data?
- Do they have an official full OpenAPI JSON export?
- Which write action is best for the first sanctioned live demo?
- Should this be public partner tooling, internal support tooling, or both?
- What PII redaction rules should be mandatory?
