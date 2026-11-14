# The resolved tokens are made from a sokkel-ui checkout's tokens.json, read and never written.
# sokkel-ui is read from .parity/sokkel-ui, a clone fetched from GitHub on every run
# (scripts/sokkel.ts): SOKKEL_UI_REF pins a commit, SOKKEL_UI_OFFLINE skips the fetch.

.PHONY: help tokens tokens-check verify-colors parity parity-web fmt fmt-check check test snapshots storybook runs tasks apps all

help:
	@echo "make tokens        resolve sokkel-ui's tokens, as GitHub has them, into plain values"
	@echo "make tokens-check  fail when they are not what sokkel-ui's tokens make"
	@echo "make verify-colors  draw sokkel-ui's CSS in Chromium and compare every resolved color"
	@echo "make parity        what changed in sokkel-ui's tasks example since it was ported, and the board"
	@echo "make parity-web    rebuild the web tasks example and shoot its screens"
	@echo "make fmt           format the workspace"
	@echo "make check         clippy with warnings denied"
	@echo "make test          every test, snapshots included"
	@echo "make snapshots     record the snapshots again (look at them before committing)"
	@echo "make storybook     run the storybook"
	@echo "make runs          run the example app"
	@echo "make tasks         run sokkel-ui's tasks example, ported one to one"
	@echo "make apps          build every example app in release and bundle each as a macOS .app in dist/apps/"
	@echo "make all           fmt-check, check and test"

tokens:
	bun scripts/resolve.ts

tokens-check:
	bun scripts/resolve.ts --check

verify-colors:
	bun scripts/verify-colors.ts

parity:
	bun scripts/parity.ts check tasks
	bun scripts/parity.ts board tasks

parity-web:
	bun scripts/parity.ts web tasks

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

runs:
	cargo run -p runs

tasks:
	cargo run -p tasks

apps:
	scripts/bundle.sh

all: fmt-check check test
