ffmt:
	cargo fix --allow-dirty --allow-staged; \
	cargo clippy --all --allow-dirty --allow-staged --fix; \
	cargo +nightly fmt;

gen:
	cargo build -p shared_types
