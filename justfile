# Comando único de checagem: define "pronto" (veja AGENTS.md).
check:
    cargo fmt --all --check
    cargo clippy --all-targets --locked -- -D warnings
    cargo test --locked
