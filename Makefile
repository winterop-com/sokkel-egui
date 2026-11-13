# The resolved tokens are made from a sokkel-ui checkout's tokens.json, read and never written.
SOKKEL_UI ?= ../sokkel-ui
# the ref sokkel-ui is read at: its main as last fetched, never the checkout's work in progress
SOKKEL_UI_REF ?= origin/main
export SOKKEL_UI SOKKEL_UI_REF

.PHONY: help tokens tokens-check verify-colors fmt fmt-check check test snapshots storybook all

help:
	@echo "make tokens        resolve the tokens of $(SOKKEL_UI) into plain values"
	@echo "make tokens-check  fail when they are not what $(SOKKEL_UI)'s tokens make"
	@echo "make verify-colors  draw sokkel-ui's CSS in Chromium and compare every resolved color"
	@echo "make fmt           format the workspace"
	@echo "make check         clippy with warnings denied"
	@echo "make test          every test, snapshots included"
	@echo "make snapshots     record the snapshots again (look at them before committing)"
	@echo "make storybook     run the storybook"
	@echo "make all           fmt-check, check and test"

tokens:
	bun scripts/resolve.ts

tokens-check:
	bun scripts/resolve.ts --check

verify-colors:
	bun scripts/verify-colors.ts

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

snapshots:
	UPDATE_SNAPSHOTS=1 cargo test --workspace

storybook:
	cargo run -p storybook

all: fmt-check check test
