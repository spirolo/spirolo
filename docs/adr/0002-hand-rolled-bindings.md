# ADR 0002: Hand-rolled binding tables instead of oxc_semantic, for now

**Date** 2026-09-18 · **Status** accepted

## Context

The initial design considered oxc's semantic analysis for scopes and bindings.
The implementation instead reads bindings with four small, purpose-built tables,
each a first-seen map keyed by name:

- `analysis.rs::BindingIndex`: every `BindingIdentifier` in a test callback, numbered
  in declaration order, so the body hash writes a local by position and a free name
  (a global, an import) by name;
- `vitest.rs::Vocabulary`: the file's import declarations, read once at the top
  level, to label the framework, pick up aliases and drop shadowed globals;
- `countable.rs::Tables`: the `const` tables a walk has passed, by name, with a
  suite-scoped stack for `describe` bodies;
- `constants.rs`: module-level string and countable `const` bindings plus the import
  and re-export bindings the whole-project fold follows.

Each answers one question for one recogniser and nothing else. `oxc_semantic` would
answer all of them with scope-accurate resolution: a `ScopeTree`, a `SymbolTable`,
and every reference bound to its declaration. That requires a full semantic build per
file inside the parallel phase, with every recogniser reading symbol ids instead of
names.

## Decision

Keep the hand-rolled tables. Do not adopt `oxc_semantic` until a
rule needs what only scope-accurate bindings give.

The original evaluation compared test counts and analysis time with runner output.
The four tables were sufficient for those cases and stay local to their recognisers.
A full semantic build would sit on the hot path of every file. Matching test counts
does not establish that binding resolution is correct for every input.

## Known limits

Recorded so nobody rediscovers them as bugs.

- **Shadowing collapses by name.** `BindingIndex` numbers the first declaration of a
  name and every later `BindingIdentifier` with the same name shares that index. Two
  sibling blocks that each declare `const x` hash as one binding; a consistent rename of
  one of them still changes nothing. The body hash therefore treats two bodies that
  differ only in *which* shadowing declaration a reference binds to as equal. For the
  duplicate-test rule this is conservative on the side of over-matching in a shape
  (same name, sibling blocks, different behaviour) that real tests rarely take.
- **A namespace import of the framework is a no-op.** `import * as vi from 'vitest';
  vi.test(…)` binds `vi`, not `test`; `Vocabulary` records only named imports, so the
  call roots in a name it does not claim and the file's cases are not recognised. The
  Vitest/Jest module docs say so. The corpus has no such file.
- **Vocabulary is file-flat.** A `test.extend` binding or an import alias is registered
  by name for the rest of the walk, whatever block declared it; a local `const test =
  …` in one `describe` is visible to a sibling `describe`.
- **Tables are first-seen.** A `const ROWS` redeclared in a nested scope is the
  outer one for counting purposes.
- **No symbol ids.** Nothing in the model can say "this reference and that one are the
  same binding" beyond the name; a rule that needs it cannot be written on the model as
  is.

## Revisit when

Revisit when a rule's precision depends on scope-accurate bindings. Possible cases:

1. `duplicate-test` if the corpus shows body-hash collisions that trace to shadowed
   names rather than to real duplicates (measure: count collisions, sample, classify);
2. `data-layer-mock` or `handler-drift` if a mock target or URL constant has to be
   followed through a local rebinding (`const client = createClient(); client.from(…)`)
   rather than an import or a module-level `const`;
3. any rule that must decide whether two references in one file are the same binding.

At that point the change is contained: `oxc_semantic` runs inside `extract_file`
while the arena is alive, `FileContext` gains the `Semantic`, and the four tables are
replaced one at a time by lookups against it, each behind the snippet tests that pin
its current answers. Take it then, and not before: adopting it now would be paying the
per-file cost for a question no rule asks yet.

## Rejected for now

- **Adopting `oxc_semantic` before a rule needs it.** The cost lands on every file
  without a consumer for the extra information.
- **A half-way scope tracker** (a block-scoped stack of names inside `BindingIndex`).
  It would fix shadowing for the hash and nothing else, and it would be a second
  scope analysis to keep correct next to the one oxc already ships.
