# Участие в host-harvester

[English version](CONTRIBUTING.md)

Соблюдайте [соглашение о качестве кода](CODE_QUALITY.ru.md) и пишите коммиты на английском по [Conventional Commits](COMMIT_CONVENTIONS.ru.md).

Запустите проверки из корня workspace.

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Используйте временные каталоги и вымышленные данные для тестов и примеров. Оставляйте данные реальных устройств за пределами коммитов и общих логов. Если меняете монтирование, постоянное хранение результатов импорта или сокет, опишите в запросе на слияние, какое поведение проверили.
