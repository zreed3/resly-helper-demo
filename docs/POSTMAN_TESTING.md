# Postman And Newman Testing

The repo includes a Postman-compatible collection that tests the CLI through a tiny local HTTP wrapper. This is useful for a demo because Resly can see familiar Postman/Newman output while the test still exercises the real `resly` binary.

## Run The Full Test

```bash
make postman-test
```

This target:

- builds `target/debug/resly`
- starts `tests/postman/server.mjs` on `http://127.0.0.1:8765`
- runs the Postman collection with Newman through `npx --yes newman`
- shuts the wrapper down after the run

## What It Checks

- fixture-mode setup with `resly --json doctor`
- manager availability quote for 3 guests from `2026-07-05` to `2026-07-07`
- scoped 2BR inventory and rates so fixture data cannot silently return the wrong room type
- refusal of a live rate update without an approval token
- refusal of a live webhook delete without `--confirm-delete`

## Manual Postman Run

In one terminal:

```bash
cargo build
node tests/postman/server.mjs
```

In another terminal:

```bash
npx --yes newman run tests/postman/resly-helper.postman_collection.json \
  --env-var baseUrl=http://127.0.0.1:8765
```

The wrapper forces safe read tests into fixture mode. The safety tests use fake test-environment credentials only far enough to prove the CLI refuses writes before any live API mutation can be attempted.
