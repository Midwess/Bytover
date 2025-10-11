PACKAGES := backend native shared shared_types

ffmt:
	@echo "==> Fixing and formatting backend"
	cargo fix -p backend --allow-dirty --allow-staged
	cargo clippy -p backend --allow-dirty --allow-staged --fix
	cargo +nightly fmt -p backend
	@echo "==> Fixing and formatting native"
	cargo fix -p native --allow-dirty --allow-staged
	cargo clippy -p native --allow-dirty --allow-staged --fix
	cargo +nightly fmt -p native
	@echo "==> Fixing and formatting shared"
	cargo fix -p shared --allow-dirty --allow-staged
	cargo clippy -p shared --allow-dirty --allow-staged --fix
	cargo +nightly fmt -p shared
	@echo "==> Fixing and formatting shared_types"
	cargo fix -p shared_types --allow-dirty --allow-staged
	cargo clippy -p shared_types --allow-dirty --allow-staged --fix
	cargo +nightly fmt -p shared_types
	@echo "==> Fixing and formatting wasm"
	cargo fix -p wasm --target wasm32-unknown-unknown --allow-dirty --allow-staged
	cargo clippy -p wasm --target wasm32-unknown-unknown --allow-dirty --allow-staged --fix
	cargo +nightly fmt -p wasm
	swiftlint lint --fix || true

web:
	cd web-next; yarn dev
