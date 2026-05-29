.PHONY: all wasm dev build test watch clean fclean re

all: build

wasm:
	cd engine-wasm && ./build.sh web dev

dev: wasm
	cd visualizer && bun run dev

build:
	cd engine-wasm && ./build.sh web release
	cd visualizer && bun run build

test:
	cd engine && cargo test

clean:
	cd engine && cargo clean
	rm -rf engine-wasm/pkg
	rm -rf visualizer/dist

fclean: clean

watch:
	@if ! command -v cargo-watch >/dev/null 2>&1; then \
		echo "Installing cargo-watch..."; \
		cargo install cargo-watch; \
	fi
	cargo watch -w engine -w engine-wasm \
		-s 'cd engine-wasm && ./build.sh web dev'

re: fclean all