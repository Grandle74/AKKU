# Modules Manager — Initial Design Notes

This document captures the current thinking on the Modules Manager and Module
Bundle design. It is not a spec — details are still open. It exists so the
idea is not lost between now and implementation.

---

## Modules Manager

Responsible for installing, removing, and managing the presence of modules
across the system. When a module is installed or removed, both the API and
the engine are made aware of the change through the Modules Manager — neither
layer manages this directly.

---

## Module Bundle

A module ships as a bundle with two distinct parts:

**API Part**
- The property definitions specific to this module.
- The properties validator for those properties.

**Engine Part**
- The execution logic.
- State helpers.
- Error awareness *(not yet implemented — planned for the current module remake).*

The bundle is unpacked on install. Each layer receives only what concerns it —
the API never touches execution logic, the engine never touches the validator.

---

## Relationship

```
                        ┌─────────────────┐
                        │ Modules Manager │
                        └────────┬────────┘
                                 │ unpacks bundle
                    ┌────────────┴────────────┐
                    │                         │
             ┌──────▼──────┐          ┌───────▼──────┐
             │  API Part   │          │ Engine Part  │
             │  validator  │          │  exec logic  │
             │  properties │          │ state helpers│
             └──────┬──────┘          └───────┬──────┘
                    │                         │
             ┌──────▼──────┐          ┌───────▼──────┐
             │     API     │          │    Engine    │
             │             │          │              │
             │ domain →    │          │ domain →     │
             │ validator   │          │ ModuleID     │
             └──────┬──────┘          └──────────────┘
                    │
             ┌──────▼──────┐
             │  Frontend   │
             │             │
             │ install /   │
             │ remove /    │
             │ manage      │
             └─────────────┘
```

The frontend sends module management intents to the API. The API communicates
with the Modules Manager to act on them.

After installation, the API holds a map of domain → validator. The engine
holds a map of domain → ModuleID. Each layer queries its own map independently
— neither reaches into the other's.

---

## Open

- The exact interface the API Part must implement is not yet decided.
- How the Modules Manager persists installed module state is not yet decided.
- Error awareness in the Engine Part is deferred to the module remake.
