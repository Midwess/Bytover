ffmt:
	cargo fix --allow-dirty --allow-staged; \
	cargo clippy --all --allow-dirty --allow-staged --fix; \
	cargo +nightly fmt \
	swiftlint lint --fix || true;

gen:
	cargo build -p shared_types

gsu:
	git submodule update --init --recursive