// TODO (requires rollback):
// Plans interrupted by Ctrl+C or process crash remain as "executing"
// indefinitely. Fix requires:
//   1. Pre-execution snapshot in executor::execute_services_plan
//   2. Rollback logic to restore from snapshot
//   3. ctrlc handler in CLI to trigger rollback + set status "canceled"
//   4. Startup scan of plans dir to detect and offer recovery for
//      any plan files still in "executing" state
//
// This needs a real planning and designing after we implement the Rollback feature
