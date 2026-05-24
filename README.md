<div align="center">
     
<img src="icon.png" alt="AKKU" width="120"/>

# AKKU
 Another Konfig & Kontrol Utility
> Declarative system configuration — plan, apply, and rollback across any OS and init system.
 
![Status](https://img.shields.io/badge/status-prototype-blue)
![License](https://img.shields.io/badge/license-LGPL--3.0-blue)
![Rust](https://img.shields.io/badge/rust-1.89.0+-orange?logo=rust)
 
</div>


## Overview

Managing system configuration today means memorizing imperative commands, manually tracking state, and hoping nothing breaks. AKKU changes that: describe the system state you want, and AKKU plans the path, applies it safely, and rolls back automatically if anything goes wrong.

Where Ansible orchestrates fleets and NixOS rebuilds entire systems, AKKU does one thing well: take any machine from its current state to your desired state — safely, step by step, through any frontend you choose.

## Why AKKU?

- **Declarative** — describe the desired state; AKKU finds the optimal path to reach it.
- **Safe by default** — automatic rollback on failure, manual rollback on demand, conflict detection before anything runs.
- **API-first** — one logic layer, fully interchangeable frontends (TUI, CLI, GUI, Web).
- **Modular** — a stable core engine communicates with pluggable modules. First-party and community modules build an ever-growing ecosystem of system capabilities.
- **System-wide** — install the module that fits your init system: systemd, OpenRC, and beyond.
- **Error-aware** — reports what failed, why it failed, and how to fix it, in plain language.

## Status

| Component                               | Status         |
|-----------------------------------------|----------------|
| Core Engine                             | ✅ Done        |
| API Layer                               | ✅ Done        |
| Conflict Detection                      | ✅ Done        |
| Declarative Syntax                      | ✅ Done        |
| Action Modes *(Normal, Force, Dry-run)* | ✅ Done        |
| Snapshots                               | ✅ Done        |
| Rollback                                | ✅ Done        |
| CLI `commando` *(reference frontend)*   | ✅ Done        |
| `systemd` Service Module                | 🔧 In Progress |
| Smart Error Awareness                   | 🔜 Planned     |
| Modules Manager                         | 🔜 Planned     |
| Third-party Modules                     | 🔜 Planned     |

## Architecture

AKKU follows a strict unidirectional layered design — each layer communicates only with the one directly below it, never sideways or upward. Frontends are fully replaceable; the engine and modules are frontend-agnostic.

```
AKKU/                    # each crate follows: 'src/' + 'docs/'
├── cli/                 # Reference frontend (commando)
├── api/                 # Orchestration layer
├── engine/              # Core engine
├── shared/              # Shared types and utilities
├── modules_manager/      # Planned
├── modules/             # Modules library
│   └── services/        # systemd service module
├── tests/               # Planned
├── ui_concept/          # Future GUI concept as HTML pages
└── README.md
```

> **Note:** This monorepo layout reflects the current development state. From v0.1 onward, each crate will live in its own repository.

## Getting Started

### Prerequisites

- `Rust 1.89.0+` — install via [rustup](https://rustup.rs/)
- Linux with systemd (current modules are systemd-based)
- Root access via `sudo` (required for actual system operations)

### Run

```bash
git clone https://github.com/Grandle74/AKKU.git
cd AKKU
cargo run
```

## Usage

```bash
# <module> <action> [target] [properties...] [flags]

# Read-only commands
commando(v0.1)~> service reset          # no target
commando(v0.1)~> service reload nginx   # targeted

# Declarative state commands
commando(v0.1)~> service cfg nginx running=1 enabled=yes    # Normal mode
commando(v0.1)~> service cfg nginx masked=1 --force         # Force mode
commando(v0.1)~> service cfg nginx running=false --dry-run  # Dry-run mode
```

## Roadmap

Track live progress on the [v0.1 Milestone](https://github.com/Grandle74/AKKU/milestone/1).

### v0.1 — Foundations
- Hardened end-to-end core (API + Engine)
- Modules Manager — initial design captured in [`ModulesManager.md`](ModulesManager.md)
- `systemd` service module reworked as a solid first-party reference
- A second lightweight module to validate the module system
- CI setup and minimum supported Rust version pinned
- CONTRIBUTING.md — architectural rules, layer contracts, contribution guidelines

### Beyond v0.1
- Each crate in its own repository under a dedicated project account
- Module ecosystem open to third-party contributions
- Early testers onboarded once the codebase reaches a stable shape
- Glossary and developer documentation

## Contributing

AKKU is approaching its first stable release. Code contributions are not open yet — the architecture is still being finalized for v0.1. That said:

- **Bug reports and feedback** are welcome via [GitHub Issues](https://github.com/Grandle74/AKKU/issues).
- **Architectural discussion** is open in Discussions.

Contributing guidelines (including layer contracts and architectural invariants) will be published alongside v0.1.

## License

This project is licensed under the [LGPL-3.0 License](LICENSE).

## Background

AKKU started as a dream to rewrite YaST2 in Rust — a beloved tool that went unmaintained. That dream gradually shifted: not a rewrite, but something new. Discovering NixOS and the declarative paradigm sharpened the vision further. The plan/apply model was arrived at independently — though it turns out [Terraform](https://terraform.io) had the same idea for infrastructure.

Conceptually inspired by [NixOS](https://nixos.org), [YaST](https://yast.opensuse.org/), and [Cockpit](https://cockpit-project.org).
