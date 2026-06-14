#!/usr/bin/env bash
set -euo pipefail

command -v resly
resly --json doctor
resly --json availability quote --guests 3 --from 2026-07-05 --to 2026-07-07 >/dev/null
resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 2 >/dev/null
resly --json webhooks create --type reservations --url https://example.com/resly/webhook --dry-run >/dev/null
echo "resly CLI is installed and fixture smoke checks passed"
