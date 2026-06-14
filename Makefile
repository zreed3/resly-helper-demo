PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: fmt test build install-local smoke postman-test

fmt:
	cargo fmt

test:
	cargo test

build:
	cargo build --release

install-local: build
	mkdir -p "$(BINDIR)"
	cp target/release/resly "$(BINDIR)/resly"
	chmod +x "$(BINDIR)/resly"

smoke:
	command -v resly
	resly --help >/dev/null
	resly --json doctor >/dev/null
	resly --json reservations list --from 2026-07-01 --to 2026-07-31 --limit 2 >/dev/null

postman-test:
	cargo build
	node tests/postman/run-newman.mjs
