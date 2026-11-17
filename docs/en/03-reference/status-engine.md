---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Status Engine

**Language:** **English** | [Русский](../ru/03-reference/status-engine.md)

## Terminology and calculation state

Terminology: In code and UI, the term In is fully replaced with Request (Запрос).

Request Statuses (Request): Satisfied / Expected (the word "Deficit" removed from UI to avoid confusing the user).

Result Statuses (Result): Expected / Surplus.

Main rule: If Request is Expected, Result is strictly blocked.

## Metric States

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

## Numeric example

A Metric can be added to absolutely any input and request of any Process. But values in each Process of this Metric are assigned individually. A Metric has the same name, ID, type, unit of measurement everywhere.

For example — in the Result of Process "Product Sales" — Revenue $10, it goes into requests of Processes "Employee 1" and "Employee 2", one of which receives $7 and the other $2, status in result "Product Sales" = Surplus (since $9 spent), status of each employee request = Satisfied.