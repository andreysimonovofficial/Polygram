---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# MCP Protocol

**Language:** **English** | [Русский](../ru/03-reference/mcp-protocol.md)

## Unified command layer

Polygram in its essence is a clean CAD with API, MCP and other connectors. The system is modular. You can create absolutely any extensions. Everything connects maximally seamlessly. Whatever extensions you connect, it is still just data transfer. Request and result. Everything is simple. Polygram just makes all this visually ergonomic and performs calculations (coefficients, formulas, etc.).

* The CAD core stores the graph, canvas, and official basic actions.
* Extensions declare tools and their commands.
* Mouse, keyboard, and MCP use one command mechanism.
* Panning, selection, and connections become extensions.
* The agent creates or updates an extension, after which it appears on the canvas.
* The vanilla core does not change when adding extensions.

## Canvas and extension management commands

MCP server (JSON-RPC 2.0 over stdio) connects to Claude Desktop, Open WebUI, and other MCP clients. Tools:

* `execute_command` — execute a Polygram command;
* `get_snapshot` — last state snapshot;
* `get_objects` — list of all objects;
* `get_stats` — graph statistics.

The agent via MCP can do `extension create` / `extension update`. Thus creating those very modules for CAD. The user feels like a real boss. Full autonomy: The agent creates/updates extension and immediately connects it to the canvas without unnecessary questions.