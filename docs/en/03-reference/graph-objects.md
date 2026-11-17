---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Graph Objects

**Language:** **English** | [Русский](../ru/03-reference/graph-objects.md)

## Process (block/node)

It is the Source of Its Goals and the Goal of Its Sources.

### Containers (lists)

* Container types:
  * **Request** — input Container. It lists everything the Process needs to work.
  * **Result** — output Container. It lists everything the Process produces.
* Records (items) in containers: Each record describes one Metric with name, value, and unit.

### Metric

It is a unit of content of both Containers. A Metric has:

* Name;
* Type: numeric, binary (✓), time, or percentage;
* Value: data source of the value: API;
* Unit of measurement;
* Fact: actual value next to the planned/requested one.

### Connection, Sources and Goals

A Connection connects a specific Result Metric with a specific Request Metric. The block on the Result side is called Source, the block on the Request side is called Goal. The same Process plays both roles relative to different neighbors:

* **Upstream** its Request is fed by Results of other blocks. It is a Goal for these Sources.
* **Downstream** its Result feeds Requests of other blocks. It is a Source for these Goals.

A Connection has throughput: a share of Result in percent or an absolute value. Therefore one Result can be split among several Goals, and one Request can be collected from several Sources.

### Department (subgraph)

* Visually looks like just a frame (stretches based on contained objects) with a header, where the name and #ID are written.
* You can select several processes and group them into a department.
* Export and import of department (subgraph) as a file.
* Can contain sub-departments, processes in them, other sub-departments in sub-departments, etc.
* Department status is determined by nested Processes and Departments and is visually displayed as its background fill.