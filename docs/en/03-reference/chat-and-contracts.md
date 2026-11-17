---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Chat and Contracts

**Language:** **English** | [Русский](../ru/03-reference/chat-and-contracts.md)

## Chat

Each Process and Department has its own chat.

In the chat you can communicate with the Agent of this Process/Department. At the same time, the Department chat is a dialog with Its Supervisor-Orchestrator. Such an Agent manages and controls the work of nested Processes and Departments in His Department.

## Example scenario

A Process has Request "Technical Specification" and output "Product for review". Then the Agent waits for the technical specification file from another Process (that is, data is transmitted between processes along connection lines), and having received it, performs the task ASAP, reporting and demonstrating a log in the chat, and when ready, He sends "Product for review" to the next Agent in the QA process. And when a bottleneck appears (for example, frequent interferences in some Process), the Supervisor-Orchestrator of the Department eliminates them. This is an AI-Corporation.

The system is not limited to only this scenario. Since Polygram is a constructor of AI-Corporations. There is not only development there. Everything is universal.

## Message

Each message is a .md file and may contain links to other messages. For example, if the Agent encountered interferences on the way in one message, He may mention a list of interferences, indicate their brief description and links to them in the final report message (which also helps the supervisor-orchestrator optimize the work of his department).

## Contract

In each chat there is a "Contract" section containing several .md files:

* Primary context with links to neighboring narrowly specialized supplements;
* Narrowly specialized supplements.

## Data transfer and tags

All messages that occur in the chat have tags. If the Agent marks a message with tag "{Metric Name}", then this message (link to .md file) is transmitted by the system along the connection line (Result Container of Source to Request Container of Goals). Other messages are more like a log of his work. They are needed for the supervisor, and also so that the user sees that the Agent is alive.

Tags are set after sending the message. Hover over the message, the # icon appears (or you can just right-click on the message), click, select a tag from the drop-down list (List of Metric Names from the Result Container).

## AI Models

Both cloud providers and local ones.
