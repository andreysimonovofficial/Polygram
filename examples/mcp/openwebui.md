# Open WebUI MCP configuration

Open WebUI supports MCP servers through its admin interface (Settings → MCP) or via environment variables.

## Option 1: Admin UI

1. Open Open WebUI → Admin Panel → Settings → MCP
2. Add a new MCP server:
   - **Name:** polygram
   - **Type:** stdio
   - **Command:** `C:\Dev\Polygram\target\debug\polygram-mcp.exe`
   - **Args:** (leave empty)

## Option 2: Environment variable

Set `MCP_SERVERS` or use the Open WebUI MCP configuration file. Format varies by version — check your Open WebUI docs.

## Using the MCP server

Once connected, you can call Polygram tools from the chat:

```
Use the execute_command tool to run Polygram commands:
{"cmd":"add_object","id":"A","name":"Процесс А","kind":"process"}
{"cmd":"add_metric_def","id":"Деньги","name":"Деньги","kind":"number","unit":"$"}
{"cmd":"add_metric_entry","object":"A","dir":"output","metric_def":"Деньги","name":"Деньги","value":100.0}
{"cmd":"connect","from_object":"A","from_entry":"Деньги","to_object":"B","to_entry":"Деньги","throughput":100.0}
```

Then use `get_objects` or `get_stats` to inspect the graph.
