---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Contributing

**Language:** **English** | [Русский](CONTRIBUTING.ru.md)

Thank you for your interest in contributing to Polygram! This document outlines the process for contributing to the project.

## How to contribute

1. Fork the repository on GitHub.
2. Create a feature branch from `main`: `git checkout -b feature/your-feature-name`
3. Make your changes.
4. Run tests: `cargo test`
5. Run clippy: `cargo clippy -- -D warnings`
6. Commit your changes with a descriptive message.
7. Push to your fork: `git push origin feature/your-feature-name`
8. Open a pull request against the `main` branch.

## Documentation conventions

All documentation follows the **Docs-as-Code** methodology with **Diátaxis** structure:

- `docs/` — product and technical documentation (English + Russian)
- `handbook/` — company knowledge base (English + Russian)
- File names: kebab-case, lowercase English, digits and hyphens only
- Links: relative Markdown links only (`[Text](../path/file.md)`)
- Every `.md` file must have YAML frontmatter and a language switcher
- Every folder must contain a `README.md` index file

## Areas of responsibility

- `src/` — Read-Write for agents (code + module documentation)
- `extensions/` — Read-Write for agents (extensions + CONTRACT.md)
- `.agent-workspace/` — Read-Write for agents (plans, logs, drafts)
- `docs/` — Read-Only for agents (technical specification source of truth)
- `handbook/` — Read-Only for agents (company strategy, business, marketing)

## Code of conduct

Be respectful and constructive. Follow the existing code style and conventions.

## License

By contributing, you agree that your contributions will be licensed under the project's license.