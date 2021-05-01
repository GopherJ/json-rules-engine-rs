install-requirements:
	@rustup component add clippy-preview
	@cargo install cargo-outdated
	@cargo install cargo-all-features
	@cargo install cargo-udeps

check:
	@cargo check-all-features
	@cargo +nightly fmt
	@cargo clippy
	@cargo +nightly udeps --all-targets
	@cargo outdated -wR
	@cargo update --dry-run

test:
	@echo -e '\e[1;31mTest in all different combination of features...\e[0m'
	@cargo test-all-features