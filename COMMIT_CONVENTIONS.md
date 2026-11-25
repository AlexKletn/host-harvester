# Commit message convention

[Русская версия](COMMIT_CONVENTIONS.ru.md)

Use [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) for commits in this repository.

Write all commit messages in English, including the description, body, and any explanatory text in footers.

```text
<type>[(<scope>)][!]: <description>

[body]

[footer(s)]
```

- Use `feat` for a new feature and `fix` for a bug fix. Other common types include `build`, `chore`, `ci`, `docs`, `style`, `refactor`, `perf`, and `test`.
- The optional scope identifies the affected area, such as `kernel` or `tools`.
- Write a short description. Add an optional body for context and optional footers for references or other metadata. Separate these sections with blank lines.
- Mark a breaking change with `!` before the colon, a `BREAKING CHANGE: <description>` footer, or both. If you use `!` alone, describe the breaking change in the commit description.

```text
build(kernel): pin Linux to 7.3.6
fix(kernel): reset PCM positions on stream reopen
docs: document code quality rules
```
