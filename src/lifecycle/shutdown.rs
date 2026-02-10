// ShutdownCoordinator was removed in v0.2.11 — it was dead code never
// instantiated from main.rs.  Graceful shutdown is handled directly in
// main.rs via signal handlers + storage.shutdown().
//
// This module is intentionally empty but kept for future use if a
// multi-component shutdown pipeline is ever needed.
