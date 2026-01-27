# YaST3 (Working Title)

A safety‑first system configuration engine designed for modern Linux environments.

YaST3 focuses on predictable, reversible system changes through explicit intent,
dry‑run planning, and built‑in rollback — without attempting to replace existing
system tools or infrastructure platforms.

---

## Why YaST3 Exists

System configuration today is often fragile, hard to reason about, and risky to
change under real conditions. Failures are common, rollbacks are manual, and
operators are forced to react after damage occurs.

YaST3 exists to make system changes safe, explainable, and reversible by design.

---

## Core Principles

- **Intent over commands**  
  Users describe what they want, not how to execute it.

- **Dry‑run by design**  
  Every change is planned and validated before execution.

- **Rollback by design**  
  Rollback is prepared before changes are applied.

- **API‑first**  
  All interactions go through a stable, explicit API.

- **GUI is optional**  
  Interfaces are replaceable; the core logic is not.

---

## High‑Level Workflow

Inspect → Plan → Dry‑run → Apply → Rollback → Report


Each step is explicit and observable.

---

## Architecture Overview

Front‑ends (CLI / GUI / Automation)
↓
API Layer
↓
Core Engine
↓
Modules
↓
Existing System Tools

For a deeper explanation, see [`docs/architecture.md`](docs/architecture.md).

---

## Example (Conceptual)

```yaml
service: nginx
state:
  installed: true
  enabled: true
  running: true
```yaml

The system:

    inspects the current state

    generates a plan

    performs a dry‑run

    applies changes if safe

    rolls back automatically on failure

Safety & Performance

Safety is achieved through planning, validation, and controlled execution — not
by slowing down operations.

The core engine is written in Rust to ensure memory safety, predictable behavior,
and high performance while reusing existing system tools.
Non‑Goals (Phase 1)

    Not a full configuration‑management replacement

    Not a cloud orchestration platform

    Not an AI‑driven decision engine

    Not a GUI‑centric tool

Project Status

Early design and architecture phase.
Current focus:

    core workflow

    safety model

    engine and module boundaries

Naming Notice

The name YaST3 is a working title and may change depending on legal and branding
considerations.


---

# 📄 `docs/architecture.md`

```md
# System Architecture

This document describes the high‑level architecture of YaST3 and the
responsibilities of its main components.

---

## Architectural Goals

- Clear separation of responsibilities
- Safety and predictability by design
- Replaceable interfaces
- Minimal coupling to system tools

---

## Component Overview

Front‑ends (GUI / CLI / Automation / Cloud)
↓
API Layer
↓
Core Engine
↓
Modules
↓
Existing System Tools


---

## Front‑ends

Front‑ends are responsible for collecting user intent and displaying results.
They contain no business logic and do not interact with the system directly.

Examples:
- CLI
- GUI
- Automation tools
- Cloud or remote controllers

---

## API Layer

The API layer is the single entry point into the system.

Responsibilities:
- validate requests
- enforce permissions
- normalize intent
- forward clean input to the core engine

---

## Core Engine

The core engine is the decision‑making center of the system.

Responsibilities:
- inspect current system state
- build execution plans
- perform dry‑runs
- control execution order
- manage rollback logic
- produce reports

The engine does not perform system changes directly.

---

## Modules

Modules translate engine plans into concrete, domain‑specific operations.

Each module handles one responsibility, such as:
- package management
- service management
- networking
- users and permissions

Modules are controlled, predictable, and explicitly allowed by the engine.

---

## Existing System Tools

Existing system tools perform the actual low‑level work.

YaST3 does not replace them.
Instead, it orchestrates them in a safe, validated, and reversible way.

---

## Execution Model Summary

- Front‑ends express intent
- API validates and forwards
- Engine plans and controls
- Modules execute safely
- System tools do the work

---

## What This Architecture Avoids

- direct UI‑to‑system access
- hidden side effects
- uncontrolled command execution
- tight coupling between components
