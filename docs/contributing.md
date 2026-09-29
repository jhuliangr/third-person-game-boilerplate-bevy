# Contributing

## Git workflow

- The main branch is `main`.
- Make small, atomic commits. Each commit should build and do one thing.
- Commit messages are short and start with a bracketed tag:

  | Tag          | Use for                                   |
  |--------------|-------------------------------------------|
  | `[ADD]`      | New features, crates, assets               |
  | `[FIX]`      | Bug fixes                                  |
  | `[UPDATE]`   | Changes to existing behavior, dependency bumps |
  | `[REFACTOR]` | Code changes without behavior changes      |
  | `[REMOVE]`   | Deleting features or files                 |
  | `[DOCS]`     | Documentation only                         |

  Examples: `[ADD] Jump ability`, `[FIX] Broken glb model`, `[UPDATE] Bevy 0.20`.

- Commit the `.blend` source and the exported `.glb` together when an asset changes.

## Code style

- Run `cargo fmt` and `cargo clippy --workspace` before committing; both must be clean.
- Comments explain *why*, not *what*. Prefer clear names over comments.
- One plugin per crate; follow the dependency direction in the
  [architecture](architecture.md).
- Record decisions that affect several crates in [decisions](decisions.md).
