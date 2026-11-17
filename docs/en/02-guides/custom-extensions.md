---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Custom Extensions

**Language:** **English** | [Русский](../ru/02-guides/custom-extensions.md)

## Working with local /extensions repositories

Extensions are stored in separate local repositories (`/extensions`).

## Autonomous creation and updating of modules by agent via MCP

Principle: Extensions are stored in separate local repositories (`/extensions`).

DNA Rule: No matter what changes are made to extensions, they never affect the clean vanilla CAD code.

Universal UI: Every UI element can be supplemented by an extension through a dynamic menu.

The agent via MCP can do `extension create` / `extension update`. Thus creating those very modules for CAD. The user feels like a real boss. Full autonomy: The agent creates/updates extension and immediately connects it to the canvas without unnecessary questions.

Mouse, keyboard, and MCP use one command mechanism.

The agent creates or updates an extension, after which it appears on the canvas. The vanilla core does not change when adding extensions.