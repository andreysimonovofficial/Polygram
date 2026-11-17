---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Core Concepts

**Language:** **English** | [Русский](../ru/01-getting-started/core-concepts.md)

## Objects

### Process (block/node)

It is the Source of Its Goals and the Goal of Its Sources.

#### Containers (lists)

* Container types:
  * **Request** — input Container. It lists everything the Process needs to work.
  * **Result** — output Container. It lists everything the Process produces.
* Records (items) in containers: Each record describes one Metric with name, value, and unit.

#### Metric

It is a unit of content of both Containers. A Metric has:

* Name;
* Type: numeric, binary (✓), time, or percentage;
* Value: data source of the value: API;
* Unit of measurement;
* Fact: actual value next to the planned/requested one.

#### Connection, Sources and Goals

A Connection connects a specific Result Metric with a specific Request Metric. The block on the Result side is called Source, the block on the Request side is called Goal. The same Process plays both roles relative to different neighbors:

* **Upstream** its Request is fed by Results of other blocks. It is a Goal for these Sources.
* **Downstream** its Result feeds Requests of other blocks. It is a Source for these Goals.

A Connection has throughput: a share of Result in percent or an absolute value. Therefore one Result can be split among several Goals, and one Request can be collected from several Sources.

#### Metric States

State is calculated automatically by connections:

1. Request (Statuses):
   1.1. Satisfied (Ok, green) = Sum of Source Results ≥ Request;
   1.2. Expected (Expected, red) = Sum of Source Results < Request;
2. Result (Statuses):
   2.1. Covered (Ok, green) = Sum of Goal Requests = Result;
   2.2. Surplus (Surplus, yellow) = Sum of Goal Requests < Result;
   2.3. Expected (Expected, red) = Sum of Goal Requests > Result and "In Progress" flag is deactivated;
   2.4. In Progress (InProgress, blue) = Sum of Goal Requests > Result and "In Progress" flag is activated;

Status priority (Which to show if multiple statuses at once. The first = highest priority):

1. Expected;
2. In Progress;
3. Surplus;
4. Covered / Satisfied;

When the request is satisfied — the process (block) performs its task ASAP.

Status is written in a specific place on the body of the Process.

*Example with metrics:*
*A Metric can be added to absolutely any input and request of any Process. But values in each Process of this Metric are assigned individually. A Metric has the same name, ID, type, unit of measurement everywhere. For example — in the Result of Process "Product Sales" — Revenue $10, it goes into requests of Processes "Employee 1" and "Employee 2", one of which receives $7 and the other $2, status in result "Product Sales" = Surplus (since $9 spent), status of each employee request = Satisfied.*

### Department (subgraph)

* Visually looks like just a frame (stretches based on contained objects) with a header, where the name and #ID are written.
* You can select several processes and group them into a department.
* Export and import of department (subgraph) as a file.
* Can contain sub-departments, processes in them, other sub-departments in sub-departments, etc.
* Department status is determined by nested Processes and Departments and is visually displayed as its background fill.