# Agent Rules

## Package Manager

- Use `pnpm` for all repo commands. Never `npm` or `yarn`.
- Exception: end-user install docs use `npm install -g` / `npm install -D`.
- Add deps without pinning versions: `pnpm add <pkg>`. Verify latest with `npm view <pkg> version` if unsure.

## Style Constraints

- No emojis anywhere in code, comments, output, or docs.
- No `--` as a dash in prose or user-facing output. Use `—` if a dash is truly needed, but prefer rephrasing. CLI flags are the only exception.
- JS files under `crates/uzumaki/js/` are V8 snapshot candidates: strict ASCII only (0x00 to 0x7F). Use `\u2014`-style escapes if a non-ASCII char is truly required.

## Comments

- Keep comments minimal. Avoid them unless genuinely required.
- No comments explaining what changed, what was fixed, or why a patch was applied. Code stands on its own.
- No comments on internal implementation unless something is genuinely non-obvious.
- JSDoc only on user-facing JS APIs that generate types.
- For internal members on user-facing APIs, default to the `private` keyword. Only fall back to `__` prefix with `@internal` JSDoc when `private` is not viable (e.g. cross-file access, runtime visibility needed).
- Never remove existing comments or commented-out code. They are intentional. Only touch them if explicitly asked.

## Coding Style

- Rust: standard `rustfmt`. 4-space indent, `snake_case`, `PascalCase` types.
- TS/TSX: Prettier, semicolons, single quotes, 2-space indent.

## Rust 2024 Idioms

- Use `if let` chains: `if let Some(x) = foo && cond` over nested `if let`.
- Prefer `let else` for early returns over nested `if let`.
- No nested `if` where a single `if let ... && ...` chain works.

## Implementation Approach

- Prefer generic, reusable solutions over one-off fixes when it is a clean win.
- Do not hardcode logic around a single bug if the underlying problem solves cleanly at a more general level.
- Do not over-engineer for hypothetical cases.
- Ask when there is real doubt. Do not make risky assumptions.
- Keep code easy to test and easy to change.
- If there is a genuinely simpler fix, do that instead of patching around the problem.

## Validation

- Use `cargo check` over `cargo build`. Faster, catches type and borrow errors.
- Default target: `cargo check -p uzumaki_runtime`.
- The `uzumaki` crate is the CLI. Only check it when changes touch CLI code.
- Do not check the whole workspace without a concrete reason.
- Add focused tests near the code you change. Rust unit tests go inline in `mod tests`.

## Documentation

Update docs after notable features, especially user-facing changes: new JS APIs, JSX props, events, or playground usage.
