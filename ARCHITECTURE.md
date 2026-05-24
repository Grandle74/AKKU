# Architecture

AKKU is a Rust workspace of five crates with a strict unidirectional
dependency rule: each layer may only import the one directly below it.
No crate reaches across or upward.

```mermaid
flowchart LR
    F[Frontends] --> API[API] --> Engine[Engine]

    Engine --> SL[Shared Libraries]
    Engine --> Mod[Modules]

    Mod --> SL
```

`Shared libraries` has zero internal dependencies — only `std` and `serde`.
Every other crate depends on it.

---

## Layers

### `shared_libs`

The cross-crate contract. Contains only types that more than one layer
must understand:

- `Domain` — identifies which system domain an Order targets
- `Action` — the three intent kinds: `Meta`, `Config`, `Custom`
- `PropertyValue` — typed value for declarative properties (`Bool`, `String`, `Number`)
- `Step` / `Steps` — a single atomic operation and the ordered sequence that forms a Plan
- `PlanSummary` / `StepSummary` — pre-processed plan records ready for frontend consumption; built by `plan_store`, never by frontends

The boundary rule is strict: only types that are genuinely part of the
shared output contract belong here. Module-specific intermediate types
live inside their own module crate. Putting them here would couple all
layers to every module's internals.

### `modules/*`

A module crate provides two distinct groups of functions to the layers above it:

**State tools** (`state_helpers`) — called by the engine's planner:
- Query the live system state for a named target
- Build a desired state from the property map supplied by the API
- Diff current against desired (producing `Delta`)
- Convert `Delta` into an ordered `Steps` list

**Execution functions** (`lib.rs`) — called by the engine's executor:
- Run imperative operations against the system
- Validate the resulting state after each operation

*(The module system is still being shaped — this was a simplified description.)*

`Delta` is currently a `pub` type — the planner calls `calc()` and
`to_steps()` with `Delta` crossing the module boundary. This is a known
pre-v0.1 issue. The initial direction is for the planner to receive a
`Steps`-producing function from each module bundle instead of
orchestrating the diff itself, making `Delta` private to its module.
The exact shape of that interface is not yet decided.

*See also: [`ModulesManager.md`](ModulesManager.md)*

### `engine`

The engine has five internal modules that communicate exclusively through
`engine/src/lib.rs`. They never import each other directly.

```
lib.rs
  ├── module_resolver  — maps Domain → ModuleId
  ├── planner          — builds Plan from Order via module state helpers
  ├── plan_store       — all filesystem I/O for ~/.akku/plans/
  ├── snapshot         — pre-execution state capture for rollback
  └── executor         — dispatches to module execution functions
```

`lib.rs` exposes four public entry points: `execute_order` (Trip 1),
`approve_plan` (Trip 2), `build_rollback_plan`, and `list_plans`.
Nothing else is public.

**Plan IDs cross crate boundaries. Plan structs do not.** The engine owns
all plan I/O; the API and frontend receive only a `PlanSummary` and pass
the `id` back for approval.

### `api`

The single entry point for all frontend calls. Owns:

- Input parsing and routing (`process_bi_intent`, `process_tri_intent`)
- Property conflict validation before the engine is ever called
- Run mode semantics (`Normal`, `DryRun`, `Force`)
- Auto-rollback on Normal-path execution failure

The API never calls module crates directly. It has no knowledge of
init system specifics, plan files, or step ordering.

### `cli`

The reference frontend (`commando` and `History`). Imports only `api`.
Never imports `engine`, `shared_libs`, or any module crate. Owns input
parsing, flag handling, and all display rendering. No execution logic
lives here.

Every frontend is responsible for shaping raw user input into the Intent
structure the API understands. The API enforces the contract; the frontend
owns the translation.

---

## The Plan / Approve Flow

Config intents (declarative desired-state) follow a two-trip path:

**Trip 1 — Plan**

```
Frontend  →  api::process_tri_intent
          →  engine::execute_order
          →  planner::create_plan   (queries module state, diffs, builds Steps)
          →  plan_store::save       (writes ~/.akku/plans/<id>.plan.json)
          ←  PlanSummary            (returned to frontend for display)
```

The in-memory `Plan` is discarded after Trip 1. The file is the handoff.

**Trip 2 — Approve**

```
Frontend  →  api::approve_intent
          →  engine::approve_plan
          →  snapshot::save         (captures pre-execution state)
          →  plan_store update      (status: "executing")
          →  executor::execute_plan (runs Steps in order, updates each step)
          →  plan_store update      (status: "completed" | "failed")
```

**Dry-run** skips both saves and both trips entirely. The plan is built
in memory and returned for display, then discarded.

**Force** collapses the two trips into one call inside the API layer —
Trip 1 runs as normal, then the API immediately approves without prompting.

---

## Rollback

### Auto-rollback

Auto-rollback is the API's responsibility, not the engine's. When
`engine::approve_plan` returns `Err` on the Normal path, the API calls
`build_rollback_plan` once and then `approve_plan` again. It never
recurses.

The `--force` path has no auto-rollback by design: the user asserted
control. The snapshot is on disk and the History TUI exposes manual undo.

### Manual rollback

The History TUI (`cli/src/history.rs`) lets the user browse past plans
and trigger a rollback interactively. It is a two-step flow: first Enter
calls `api::preview_rollback_intent`, which builds the rollback plan and
returns a `PlanSummary` for review. Second Enter calls `api::approve_intent`
to execute it. Esc cancels and rejects the pending plan. No rollback logic
lives in the TUI — it is display and routing only.

### Shared mechanics

A rollback is a normal Config plan constructed from a snapshot rather
than from a user intent. It travels the same plan/approve path as any
forward change — no rollback-specific execution logic exists.

**Snapshot capture** happens in `approve_plan` before execution begins,
for every non-rollback plan. It serialises the live system state to
`~/.akku/snapshots/<plan_id>.snapshot.json` as an untyped JSON blob.
The untyped form means one file format serves all modules without a
shared trait or enum wrapper.

**Rollback plan construction** (`build_rollback_plan`) loads the
snapshot, translates it back to a typed property map, and feeds it to
the normal planner. The planner produces Steps exactly as it would for
any forward change.

**The snapshot gate** prevents infinite loops: `approve_plan` checks
`plan.rollback_of.is_none()` before capturing. A rollback plan never
triggers another snapshot.

---

## Validation Split

Conflict validation is intentionally split across two layers:

**API validator** — catches logically impossible desired states with no
system knowledge required. Example: `enabled=true` + `masked=true` is
impossible regardless of current state. This runs before the engine is
called, so no plan file is ever written for a provably invalid request.

**Module real-state validator** — catches impossible transitions by
comparing desired state against actual current system state. Only the
module can perform this check because only the module knows how to read
its specific live state. This layer is part of the module
bundle design and is not yet fully implemented; it is planned as part of
the Modules Manager work ahead of v0.1.

The two validators serve different purposes and must not be merged.

---

## Module System

The module system design is still being finalised ahead of v0.1. The
current services module is a working prototype; it will be reworked as
part of the path to a stable first release. The concrete interface that
modules must implement — including how the Modules Manager installs them,
how each layer receives its part of the bundle, and how the planner
decouples from module internals — will be locked down at v0.1.

What is already stable: the layered dependency rule, the `Steps` output
contract, and the separation between the API validator and the module
real-state validator.

---

## Key Invariants

- `shared_libs` has no internal dependencies.
- Engine internals communicate only through `engine/src/lib.rs`.
- Plan structs never leave the engine. Only IDs and `PlanSummary` cross the boundary.
- Snapshot capture is skipped for rollback plans (`plan.rollback_of.is_none()`).
- Auto-rollback is the API's responsibility. The engine executes; it does not recover.
- No logic lives in the frontend. Display is the frontend's only job.
- `Steps` is the only type a module owes the engine. How the module
  reaches it — through `Delta` or any other internal type — is that
  module's private concern and must never move into `shared_libs`.

## Glossary

| Term | Definition |
|------|------------|
| **Domain** | The system area an operation targets (e.g. `Services`). Maps 1-to-1 to a module crate. |
| **Action** | The kind of operation requested — `Meta` (no target), `Config` (declarative), or `Custom` (imperative). |
| **Order** | The fully parsed instruction assembled by the API and handed to the engine. Carries domain, action, target, properties, and run mode. |
| **Properties** | Key-value pairs attached to a `Config` intent that declare the desired state (e.g. `running=true`). Parsed by the CLI, validated by the API, and consumed by the planner during diffing. |
| **Intent** | A user's raw input before it becomes an Order. Classified as bi-intent (2 tokens) or tri-intent (3+ tokens). |
| **Bi-intent** | A two-token command with no target — domain + Meta action only (e.g. `service list`). |
| **Tri-intent** | A three-or-more-token command that includes a target, optionally with properties (e.g. `service cfg nginx running=true`). |
| **Delta** | The diff between a target's current state and its desired state. Produced by the planner; consumed by step generation. |
| **Step** | A single atomic operation within a Plan (e.g. `enable nginx`). Steps are ordered and cannot be reordered by callers. |
| **Plan** | An ordered list of Steps awaiting user approval. Persisted to disk as a `.plan.json` file in `~/.akku/plans/`. |
| **PlanSummary** | A pre-processed, display-ready view of a Plan. Built by `plan_store`; frontends render it without transforming it. |
| **Snapshot** | The captured pre-execution state of a target, saved before a Plan runs. Persisted to `~/.akku/snapshots/`. The source of truth for rollback. |
| **RunMode** | How a Config intent is handled after planning — `Normal` (prompt), `DryRun` (show only), or `Force` (auto-approve). |
| **Module** | A crate implementing domain and system specific logic (state querying, step execution). Multiple modules can serve the same domain — e.g. a `services-systemd` and `services-openrc` module both serving the `Services` domain. |
| **ModuleId** | The engine-side identifier for an installed module. Resolved from a Domain by `module_resolver`. |
| **commando** | The developer/tester reference CLI. Not a consumer-facing tool — a reference implementation for how frontends call the API. |
| **History** | The persisted record of all Plans, accessible via the `history` command. Displayed in an interactive TUI that supports manual rollback. |
