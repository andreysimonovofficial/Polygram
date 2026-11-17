---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# CAD Core Design

**Language:** **English** | [Русский](../ru/04-architecture/cad-core-design.md)

## Clean vanilla CAD core philosophy

Polygram in its essence is a clean CAD with API, MCP and other connectors. The system is modular. You can create absolutely any extensions. Everything connects maximally seamlessly. Whatever extensions you connect, it is still just data transfer. Request and result. Everything is simple. Polygram just makes all this visually ergonomic and performs calculations (coefficients, formulas, etc.).

* The CAD core stores the graph, canvas, and official basic actions.
* Extensions declare tools and their commands.
* Mouse, keyboard, and MCP use one command mechanism.
* Panning, selection, and connections become extensions.
* The agent creates or updates an extension, after which it appears on the canvas.
* The vanilla core does not change when adding extensions.

## Dynamic UI menus

Universal UI: Every UI element can be supplemented by an extension through a dynamic menu.