PLUGIN_NAME = files
CRATE_WASM  = target/wasm32-wasip1/release/files.wasm
WASM_OUT    = $(PLUGIN_NAME).wasm

.PHONY: all build frontend wasm test lint clean sign verify release info

all: build

build: frontend wasm

frontend:
	cd frontend && npm ci && npm run build

wasm:
	cargo build --target wasm32-wasip1 --release
	@if command -v wasm-opt >/dev/null 2>&1; then \
		wasm-opt -Oz --enable-bulk-memory --enable-sign-ext --enable-mutable-globals \
			--enable-nontrapping-float-to-int -o $(WASM_OUT) $(CRATE_WASM); \
	else \
		echo "wasm-opt not found; install binaryen for smaller output"; \
		cp $(CRATE_WASM) $(WASM_OUT); \
	fi

test:
	cargo test
	cd frontend && npm test

lint:
	cargo clippy --target wasm32-wasip1 -- -D warnings
	cargo clippy --all-targets -- -D warnings
	cd frontend && npm run typecheck

sign:
	@if [ -n "$(GPG_KEY)" ]; then \
		gpg --detach-sign --armor --local-user $(GPG_KEY) $(WASM_OUT); \
	else \
		gpg --detach-sign --armor $(WASM_OUT); \
	fi

verify:
	gpg --verify $(WASM_OUT).asc $(WASM_OUT)

release: build sign

clean:
	cargo clean
	rm -f $(WASM_OUT) $(WASM_OUT).asc
	rm -rf frontend/dist

info:
	@echo "Plugin:  $(PLUGIN_NAME)"
	@echo "Version: $$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
