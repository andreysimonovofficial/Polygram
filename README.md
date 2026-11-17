---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Polygram

**Language:** **English** | [Русский](README.ru.md)

Visual graph editor for orchestrating AI-Corporations with 10,000+ simultaneously operating AI agents and live employees, generating real monetary profit.

Polygram is a living map of any system (factory, company, data center, household): objects with metrics, connected by flows. It is visible where something is lacking, where there is surplus, and where the bottleneck is. The map can be managed by both a human and an AI agent — on equal terms.

With Polygram you can create and finely tune **AI-Corporations** from 10,000+ simultaneously operating AI agents and live employees that generate real monetary profit.

---

## What it is

Polygram separates **computation** from **window**. The Rust core works without graphics — on a server, in a data center, in the background at the agent. The egui window is just a thin shell that draws what the core has already calculated.

All changes go through **commands** (`Command`, serialized as JSON). All results are **events** (`Event`) with a revision number. Window, MCP, HTTP API, and console terminal are equal: each speaks the same command language.

> **One door.** A human (menu), an agent (MCP tools), and the outside world (Google Sheets, Analytics, Monobank, etc.) enter through a single API.

---

## Who it is for

| Role | What they get from Polygram |
|------|-----------------------------|
| **Business owner** | Visual model of an AI-Corporation: departments, processes, profit metrics, bottlenecks in real time |
| **Department head** | Map of their department with metrics, department chat, and change history |
| **AI engineer** | API, MCP server, and JSONL scripts for managing agents and connectors |
| **Developer** | Rust core without UI dependencies, testable in isolation, regression on JSONL |

---

## Key capabilities

### Graphical corporate model
- **Departments, processes, metrics** — hierarchy in three separate stores (structure, values/statuses, geometry)
- **Flows between objects** — connection of one object's result to another's request with throughput coefficient
- **Metric statuses** — `Ok`, `Surplus`, `In Progress`, `Expected` — calculated incrementally on each change

### 10,000+ objects
Target performance reserve — tens of thousands of objects and connections. The core assembles the graph in milliseconds, recalculates only marked parts, publishes immutable snapshots for clients.

### Unified command language
One command — one JSON string. Works the same from GUI, CLI, MCP, and HTTP API:
```json
{"cmd":"add_object","id":"sales","name":"Отдел продаж","kind":"department"}
{"cmd":"add_metric_def","id":"profit","name":"Прибыль","kind":"money","unit":"$"}
{"cmd":"add_metric_entry","object":"sales","dir":"output","metric_def":"profit","name":"Прибыль","value":5000}
{"cmd":"connect","from_object":"marketing","from_entry":"leads","to_object":"sales","to_entry":"leads","throughput":100}
```

### MCP server (for AI agents)
JSON-RPC 2.0 over stdio. Connects to Claude Desktop, Open WebUI, and other MCP clients. Tools:
- `execute_command` — execute a Polygram command
- `get_snapshot` — last state snapshot
- `get_objects` — list of all objects
- `get_stats` — graph statistics

### HTTP API
Port 8080. Endpoints: `GET /api/snapshot`, `GET /api/stats`, `GET /api/objects`, `POST /api/commands`.

### CLI terminal `pg`
```powershell
pg bench 10000                    # speed test: 10,000 objects
pg run examples/demo.jsonl        # run command scenario
pg run examples/demo.jsonl --save corp.pg   # save corporation to file
pg open corp.pg                   # project summary and bottlenecks
```

### Visual canvas (egui)
- GPU canvas with camera, zoom, and panning
- Levels of detail and automatic layout
- Straight and Z-shaped connection lines (built on the canvas side, the core does not store them)
- Changeable theme: contrast, fast, straight lines (later — glamorous with Bezier curves)
- Virtual mouse and keyboard for agent control
- Built-in canvas screenshots (`View → Screenshot`)

### `.pg` — portable department
The entire `.pg` file is a whole "corporation" (departments, processes, settings, connectors) that is imported as a department into another system. Example: "AI-corporation on a flash drive."

### Chat by hierarchy
Each department has its own chat. Object and metric changes appear in the parent chat as widget messages. The agent sees only the context for their task — the sub-agent is blind to other areas.

---

## Architecture (three stores)

```
┌─────────────────────────────────────────┐
│  Command (JSON)  ◄──  GUI / CLI / MCP   │
│                                      │    │
│  Engine (one thread)                  │    │
│  ├ Hierarchy  — departments, processes │    │
│  ├ Values  — numbers and statuses      │    │
│  └ Geometry — card coordinates        │    │
│                                      │    │
│  Event (revision) ──► Snapshot for UI │    │
└─────────────────────────────────────────┘
```

1. **`core`** — core: hierarchy, values and statuses, card grid, commands, events, bottlenecks. Does not know about window, graphics, and network.
2. **`cli`** — console terminal `pg`, running the core without a window, benchmarks.
3. **`canvas`** — GPU canvas, camera, levels of detail, theme switching, line routing.
4. **`polygram`** — egui window: menu, command palette, status bar, canvas.
5. **`api`** — HTTP and MCP, context slices, rights and scopes.
6. **`mcp`** — MCP server (JSON-RPC 2.0 over stdio) for Claude Desktop and others.

Rules:
- Everything changes only by command
- Everything that happened — an event with a revision
- One data owner (engine in its thread), others read snapshots
- Derived data is not stored (statuses and flows are recalculated; lines are built by the canvas based on card positions)

---

## Quick start

### Build
```powershell
cargo build --release
```

### Run application
```powershell
cargo run --release -p polygram
```

### Demo (mini-factory)
```powershell
# Build and run demo scenario
cargo run --release -p polygram-cli -- run examples/demo.jsonl --save demo.pg

# Add power (bottlenecks disappear)
cargo run --release -p polygram-cli -- run examples/demo_fix.jsonl --load demo.pg
```

### Performance benchmark
```powershell
cargo run --release -p polygram-cli -- bench 10000
```

### All tests
```powershell
cargo test -p polygram-core        # 34+ core checks
cargo test                         # all workspace tests
```

---

## Example: AI-corporation in 3 steps

1. **Create departments** — marketing, sales, support, finance
2. **Add metrics** — leads, deals, revenue, profit ($), NPS
3. **Connect by flows** — marketing → sales → revenue → profit

Now every deal, every lead, and every dollar is displayed on the graph in real time. Bottlenecks are highlighted automatically. Agents manage via MCP, humans — via menu and canvas.

---

## Technology stack

- **Language:** Rust (edition 2021)
- **GUI:** egui 0.37 + eframe
- **Serialization:** serde + serde_json
- **HTTP API:** reqwest + tokio
- **MCP:** JSON-RPC 2.0 over stdio
- **Testing:** built-in `test` + regression on JSONL

---

## Documentation

| File | Description |
|------|-------------|
| `docs/VISION.md` | Project vision |
| `docs/ARCHITECTURE.md` | Architecture: rooms and doors |
| `docs/TESTING.md` | Automated regression testing |
| `docs/DECISIONS/` | ADR decisions (0001–0011) |
| `examples/mcp/README.md` | MCP server and connection |
| `docs/en/` | Product & technical documentation (English, Diátaxis) |
| `docs/ru/` | Product & technical documentation (Russian, source of truth) |
| `handbook/en/` | Company handbook (English) |
| `handbook/ru/` | Company handbook (Russian, source of truth) |

---

## License

[LICENSE](LICENSE)

---

With Polygram your AI-Corporation is not a diagram, but a working system that counts money in real time.