# COMMENTING.md — AKKU Commenting Guidelines

Comments are part of the codebase. They get stale, they lie, they accumulate.
Write fewer of them, and write them to last.

---

## The Core Rule

**Comments explain *why*. Code explains *what*.**

Before writing a comment, ask: *would a competent Rust contributor reading
this file wonder why this decision was made?* If yes, explain the decision.
If the code already answers the question by itself, stay quiet.

---

## When to Comment

Write a comment when the code cannot answer these questions on its own:

- **Why this approach over another?**
  A constraint, a trade-off, a platform quirk that forced the hand.

- **Why is something intentionally absent?**
  A missing field, a skipped check, a path with no auto-rollback — absence
  is invisible and therefore needs explaining.

- **Why is this ordering fixed?**
  Step sequences, lifecycle transitions, two-pass validations — when order
  matters and the code doesn't enforce it structurally.

- **What does this value actually mean?**
  Domain knowledge that lives outside the codebase: systemd quirks, OS
  behaviour, a protocol edge case. The code cannot express this; a comment can.

Do not write a comment when:

- The identifier name already communicates the intent.
- The comment would be a prose translation of the line below it.
- The information is already in the type signature or return type.

---

## File Headers

Every source file opens with its path and a short header block.

```rust
// crate/src/path/to/file.rs
//
// One sentence: what this module's job is within the architecture.
//
// [What this module does NOT own — when the boundary is non-obvious.]
//
// [One key invariant or constraint that governs the whole file, if any.]
```

**The "does NOT" line is the most important part.** It stops the next
contributor from adding logic that belongs in another layer.

Keep it tight. Do not list every function the file contains — that's what
doc comments are for and it will go stale. Do not describe the full
architecture — that belongs in `ARCHITECTURE.md`.

**Example:**

```rust
// engine/src/plan_store.rs
//
// All filesystem operations for Plan persistence.
//
// This is the single owner of the ~/.akku/plans/ directory.
// No other module reads or writes plan files.
```

---

## Public API Doc Comments (`///`)

Every `pub` item that is part of a crate's external interface gets a `///`
doc comment. `pub(crate)` and private items don't require one unless there is
something genuinely non-obvious to say.

**First line:** a single imperative sentence. Not "This function returns..." —
the reader knows it is a function.

**Follow with a blank `///` line and further paragraphs only when needed:**

- A constraint the caller must respect.
- A design decision that affects callers.
- What is intentionally absent from the inputs or outputs.

Do not add `# Arguments` / `# Returns` sections unless the type signature
alone is genuinely ambiguous. A `Result<Vec<String>, Vec<String>>` where
errors are human-readable lines is self-documenting.

**Example:**

```rust
/// Trip 2: forwards the user's approval decision to the engine.
///
/// On execution failure, the API triggers auto-rollback — not the engine.
/// This is the Normal path only. --force has no auto-rollback by design:
/// the user asserted control, so failure is left for manual resolution.
///
/// Rejection returns Ok, not Err — the user made a valid choice.
pub fn approve_intent(id: &str, approved: bool) -> IntentOutcome {
```

Three things earned their place: what it does, the auto-rollback ownership
decision, and the non-obvious `Ok` on rejection. Nothing else is needed.

---

## Internal Logic Comments (`//`)

Use `//` inside function bodies only when the reasoning behind a decision
would not be obvious to a competent reader.

The bar: *would someone maintaining this code wonder "why?" at this line?*
If yes, answer it. If no, skip it.

**Good — explains a non-obvious constraint:**

```rust
// "static" means the unit has no [Install] section but is enabled in practice.
enabled: matches!(unit_file_state.as_str(), "enabled" | "static"),
```

**Good — explains a silent failure mode:**

```rust
// Reject duplicate keys here — HashMap::insert silently drops the first value.
if properties.contains_key(key) {
```

**Good — explains why something is intentionally absent:**

```rust
// plan_text is not threaded through — the CLI already rendered it before
// the approval prompt. Including it here would cause a double-print.
fn build_rollback_outcome(plan_id: &str, exec_errors: Vec<String>) -> IntentOutcome {
```

**Bad — narrates the code:**

```rust
// Pass 1: per-property type validation.
for (key, value) in properties {
    validate_property(key, value)?;
}
```

Delete it. The code says this.

**Inline on match arms** works well for the non-obvious branch only:

```rust
mode: match (is_config, mode) {
    (true, RunMode::Normal) => Some("normal".to_string()),
    (true, RunMode::Force)  => Some("force".to_string()),
    _                       => None, // DryRun or non-Config: don't save
},
```

Comment the surprising arm. Leave the clear ones alone.

---

## Implementation Specificity

The core layers — shared_libs, engine, API — are implementation-agnostic by
design. A comment that names a concrete frontend, init system, or tool leaks
an assumption the code doesn't make. When that implementation changes, the
comment becomes a lie.

Match the vocabulary the code actually uses.

**Bad:**
```rust
// The CLI already rendered the plan before the approval prompt.
```

**Good:**
```rust
// The frontend already rendered the plan before the approval prompt.
```

**Exception:** modules already scoped to a specific system — a `systemd`
module, an `openrc` module — may name the concrete tool when the behaviour
they're explaining is specific to it.

---

## Section Dividers

Use section dividers to group related items within a file. The format is fixed:

```rust
// ── Section Name ──────────────────────────────────────────────────────────────
```

- Trailing `─` characters fill to column 80.
- Title case.
- Inside files, never inside functions.
- Only when there are at least two meaningful groups to separate.

Dividers are navigational — they are not documentation. Do not put
explanatory text under a divider; that belongs on the items themselves.

---

## TODOs

TODOs are for known limitations or deliberate trade-offs, not for finishing
work that should be done now.

```rust
// TODO: Return structured ServiceEntry data instead of formatted strings.
// Formatted strings work for the CLI but block future GUI/TUI frontends
// from rendering their own layouts.
```

A TODO without a "why it hasn't happened yet" is noise. File an issue instead.

---

## Maintaining Comments

A comment that no longer matches the code it sits next to is worse than no
comment — it actively misleads.

When you change code, ask:

1. **Did the *why* change?** If you changed the approach, not just the
   implementation, update the comment to reflect the new reasoning.

2. **Did you make something absent that was present, or vice versa?**
   Comments explaining intentional absence need to be added, updated, or
   removed accordingly.

3. **Did a constraint change?** If a systemd quirk is fixed, a protocol
   decision is revised, or a business rule is dropped — remove the comment
   that existed because of it.

4. **Did the file's responsibility shift?** If a module now owns something
   it didn't before (or gave something up), update the header.

You do not need to comment every change. Most changes speak for themselves.
The test is simple: *if I read this comment alongside this code tomorrow,
will it still be true?* If the answer is no, fix it before the PR.

---

## Consistency

- Every file starts with `// crate/src/path.rs` on line 1. No blank line before it.
- `///` for public interface items. `//` for everything else.
- Commented-out code is not committed. Placeholder stubs for unimplemented
  modules are the exception — they must carry a comment explaining what they
  are waiting for.
- No `/// # Arguments` / `/// # Returns` sections unless the type is genuinely
  ambiguous without them.
