# Contributing to host-harvester

[Русская версия](CONTRIBUTING.ru.md)

Follow the [code quality agreement](CODE_QUALITY.md) and write [Conventional Commits](COMMIT_CONVENTIONS.md) in English.

Run these checks from the workspace root.

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Use temporary directories and synthetic input for tests and examples. Keep data collected from real devices out of commits and shared logs. For changes to mounting, persistent imports, or the socket, describe the behavior you verified in the pull request.
