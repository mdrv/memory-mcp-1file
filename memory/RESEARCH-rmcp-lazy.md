RESEARCH: rmcp Lazy Initialization
ID: RES-20260112-rmcp
Status: completed
Goal: Determine if rmcp supports lazy tool registration and startup before dependencies are ready.
Path: doc/research/rmcp_lazy_init.md
Updated: 2026-01-12T20:00:54Z

Open Questions:
- [x] Can serve_server be started before all dependencies are ready? (Yes)
- [x] Is there a way to delay tool registration or make it lazy? (Yes, via custom Service)
- [x] How does the MCP handshake work? (Handshake first, tools later)
- [x] Are there examples of lazy initialization in MCP servers? (Conceptually supported via notify_tool_list_changed)

Conclusions (Findings):
- serve_server handles handshake independently of tool availability.
- Custom Service implementation with interior mutability is needed for dynamic tool registration.
- notify_tool_list_changed is the key mechanism to trigger client refresh after lazy loading.

Approved Decisions:
- Use custom Service implementation for servers requiring lazy initialization.
