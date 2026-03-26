# YaST3
> **Codename: Project ANU** — A declarative system configuration engine written in Rust.

[![Status](https://img.shields.io/badge/status-active%20prototype-orange.svg)]()
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-TBD-lightgrey.svg)]()

---

## What Is This?

YaST3 is a **system configuration engine** that lets you express the *desired state* of your system rather than issuing imperative commands. You describe what you want; the engine figures out how to get there safely.

```
# Imperative (old way)
$ systemctl enable nginx
$ systemctl start nginx

# Declarative (YaST3 way)
> service nginx running=true enabled=true
```

The engine inspects current state, plans the minimal steps to reach the desired state, validates for conflicts before touching anything, and executes in the correct order.

---

## Architecture

```
Frontend (CLI)
     ↓
  API Layer          — intent parsing, input validation, conflict detection
     ↓
  Core Engine        — state inspection, planning, execution
     ↓
  Modules            — domain-specific logic (services, network, users…)
     ↓
  System Tools       — systemctl, ip, useradd, etc.
```

The frontend is **replaceable by design**. `commando` (see below) is the reference CLI used during development — but any interface that speaks to the API layer works.

---

## Current State

| Component            | Status                          |
|----------------------|---------------------------------|
| CLI (`commando`)     | ✅ Working                      |
| API layer            | ✅ Working                      |
| Core engine          | ✅ Working                      |
| Services module      | ✅ Working                      |
| Declarative syntax   | 🔄 In progress                  |
| Conflict detection   | 🔄 In progress                  |
| Rollback             | ❌ Not yet implemented          |
| Other modules        | ❌ Not yet implemented          |

**What works today:**

```
> service list
> service status nginx
> service start nginx
> service enable nginx
> service stop nginx
> service disable nginx
> service mask nginx
> service unmask nginx
```

**What's being added:**

```
# Declarative state (specify end state, not actions)
> service nginx running=true enabled=true

# Conflict detection (validated before execution)
> service nginx enabled=true masked=true
✗ Conflict: cannot enable a masked service. Unmask it first.
```

---

## Design Goals

### 1. Declarative over Imperative
Users specify the desired end state. The engine resolves the correct sequence of actions automatically — including ordering (e.g. unmask → enable → start) and avoiding impossible states.

### 2. Conflict Detection Before Execution
Impossible or contradictory states are caught at the API layer before any system call is made.

```
enabled=true  +  masked=true    → error
running=true  +  masked=true    → error
```

### 3. Generalized Module System
The API and engine are domain-agnostic. The services module is the first implementation, but the same pipeline handles any future module — network, users, firewall — without changes to the core.

### 4. Safe Execution
Before touching the system, the engine knows:
- What the current state is
- What steps are needed
- In what order
- What rollback would look like *(planned)*

---

## `commando` — The Reference Frontend

`commando` is a minimal CLI that exercises the full stack. It is **not** the final user-facing interface — it exists so that:

- Developers can test the engine and modules directly
- Module authors have a working reference to build against
- The frontend contract stays honest (if it's hard to use via CLI, the API is wrong)

```bash
cargo run --bin commando
commando(v0.1)~> service status nginx
commando(v0.1)~> service nginx running=true enabled=true
```

Anyone building a frontend (TUI, web UI, daemon, etc.) should use `commando` as the reference for how the API behaves.

---

## Getting Started

### Prerequisites
- Rust 1.75+ — install via [rustup](https://rustup.rs/)
- Linux with systemd
- `sudo` access (required for actual system operations)

### Build & Run

```bash
git clone <repo-url>
cd yast3

cargo build

# Launch the reference CLI
cargo run --bin commando
```

> ⚠️ Commands that modify system state (start, enable, mask, etc.) require sudo. `status` and `list` do not.

---

## Project Structure

```
yast3/
├── cli/              # commando — reference frontend
├── api/              # Intent parsing, validation, conflict detection
├── engine/           # Planner, executor, state inspection
└── modules/
    └── services/     # systemd service management
```

The plan is to eventually split these into separate repositories under a dedicated project organization once the architecture is stable.

---

## What's Next

- **Declarative syntax** — full support for `service <name> key=value` style
- **Conflict validation** — enforce impossible state rules at API level
- **Rollback** — generate and store recovery steps during planning phase
- **Network module** — first expansion beyond services
- **Proper error types** — current error handling is too generic

---

## Honest Limitations

This is an active prototype. Known gaps:

- Rollback is not implemented yet — failed operations do not auto-recover
- Only the services module exists — no cross-module coordination
- Sequential execution only — no parallelism
- Error messages need work — some are still too generic

These are known priorities, not surprises.

---

## Inspiration

The design draws from:
- **Terraform** — plan/apply workflow and explicit state management
- **NixOS** — declarative configuration, atomic changes
- **Ansible** — idempotency and check mode (dry-run)
- **Kubernetes** — desired state reconciliation

The key difference: YaST3 focuses on **pre-execution validation and planning** rather than post-execution reconciliation. Conflicts are caught before the system is touched.

---

## Status Note

This repository is private and under active development. The architecture is stabilizing but not finalized. The naming situation is being handled separately — for now, **YaST3** is the working name and **ANU** is the project codename.
