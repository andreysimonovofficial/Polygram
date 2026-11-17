---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# ADR 0002: Request/Result Terminology

**Language:** **English** | [Русский](../ru/04-architecture/adr/0002-request-result-terminology.md)

## Status

Accepted

## Context

The term In was used in code and UI, which caused confusion among users. It is necessary to fix the replacement of In with Request (Request) and removal of the word "Deficit" from UI.

## Considered Options

1. Leave the term In — rejected, as it causes confusion.
2. Replace In with Request (Request) and remove "Deficit" from UI — accepted.

## Decision

The term In is fully replaced with Request (Request). Request Statuses: Satisfied / Expected (the word "Deficit" removed from UI). Result Statuses: Expected / Surplus. Main rule: If Request is Expected, Result is strictly blocked.

## Consequences

* Positive: clear terminology, absence of user confusion.
* Negative: need to update all references to the term In in code and documentation.

## Date

2026-11-17

## Author

Andrii Simonov
