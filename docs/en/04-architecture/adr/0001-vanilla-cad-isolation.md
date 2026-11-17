---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# ADR 0001: Vanilla CAD Core Isolation

**Language:** **English** | [Русский](../ru/04-architecture/adr/0001-vanilla-cad-isolation.md)

## Status

Accepted

## Context

When adding extensions to `/extensions`, it is necessary to guarantee that the clean vanilla CAD core code does not change. Extensions declare tools and their commands, but never affect the core.

## Considered Options

1. Allow extensions to modify the CAD core — rejected, as it violates modularity and stability principles.
2. Complete isolation of the core from extensions — accepted. Extensions are stored in separate local repositories (`/extensions`) and interact with the core only through the command mechanism.

## Decision

The vanilla core does not change when adding extensions. Extensions declare tools and their commands. Mouse, keyboard, and MCP use one command mechanism. The agent via MCP can do `extension create` / `extension update`.

## Consequences

* Positive: core stability, modularity, ability to safely add new features.
* Negative: extensions cannot directly change core internal logic.

## Date

2026-11-17

## Author

Andrii Simonov
