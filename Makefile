PACKAGES := backend native shared shared_types

ffmt:
	for pkg in $(PACKAGES); do \
		echo "==> Fixing and formatting $$pkg"; \
		cargo fix -p $$pkg --allow-dirty --allow-staged; \
		cargo clippy -p $$pkg --allow-dirty --allow-staged --fix; \
		cargo +nightly fmt -p $$pkg; \
	done; \
	swiftlint lint --fix || true

gen:
	cargo build -p shared_types

gsu:
	git submodule update --init --recursive

web:
	cd web-next; yarn dev
