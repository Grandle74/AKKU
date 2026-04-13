## Handle Interrupted Plans

**Issue:** Plans stuck in "executing" state when interrupted by Ctrl+C or crash.

**Fix needed:**
1. Pre-execution snapshot in `executor::execute_services_plan`
2. Rollback logic to restore from snapshot
3. Ctrl+C handler in CLI to trigger rollback + set status "canceled"
4. Startup scan to detect and recover plans in "executing" state

**Depends on:** Rollback feature implementation first