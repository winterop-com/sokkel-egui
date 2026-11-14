# The resolved tokens are sokkel-design's, copied from .parity/sokkel-design (scripts/design.ts),
# and sokkel-ui is read from .parity/sokkel-ui (scripts/sokkel.ts): clones fetched from GitHub on
# every run, read and never written.

.PHONY: help tokens tokens-check parity parity-web fmt fmt-check check test snapshots storybook runs tasks status apps all

help:
	@echo "make tokens        copy sokkel-design's resolved tokens, as GitHub has them, into the core crate"
	@echo "make tokens-check  fail when the copy is not sokkel-design's"
	@echo "make parity        what changed in sokkel-ui's tasks example since it was ported, and the board"
	@echo "make parity-web    rebuild the web tasks example and shoot its screens"
	@echo "make fmt           format the workspace"
	@echo "make check         clippy with warnings denied"
	@echo "make test          every test, snapshots included"
	@echo "make snapshots     record the snapshots again (look at them before committing)"
	@echo "make storybook     run the storybook"
	@echo "make runs          run the example app"
	@echo "make tasks         run sokkel-ui's tasks example, ported one to one"
	@echo "make status        run sokkel-ui's status example, being ported one to one"
	@echo "make apps          build every example app in release and bundle each as a macOS .app in dist/apps/"
	@echo "make all           fmt-check, check and test"

tokens:
	bun scripts/design.ts

tokens-check:
	bun scripts/design.ts --check

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

status:
	cargo run -p status

apps:
	scripts/bundle.sh

all: fmt-check check test
