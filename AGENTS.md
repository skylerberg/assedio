# Assedio

Assedio is a two-player siege game on a 9×7 board. Complete rules, including
every interpretation chosen where the rules sheet is silent, are in `RULES.md`.
Code and interface use the sheet's terms: the March and Flank tokens, each with
a Move side and an Attack side, rows and columns, Attack Zones, and rocks.

## Tech Stack

- Rust (game engine and AI), Svelte 5 + TypeScript (frontend), Vite (build)
- Monte Carlo tree search via the `mcts` crate from the `monte_carlo`
  repository, a git dependency pinned to a commit in the workspace root

## Project Structure

Rust workspace plus a Svelte frontend:

- `assedio-core/` — the whole game engine: `types.rs` (board, pieces,
  tokens, state), `rules.rs` (setup, legal choices, turn resolution),
  `search.rs` (the `mcts::Game` impl and the leaf evaluation)
- `assedio-wasm/` — wasm-bindgen bindings, JSON in and JSON out
- `assedio-runner/` — CLI for simulation, replay and benchmarking
- `src/` — Svelte 5 + TypeScript frontend
  - `src/data/` — TypeScript mirror of the wire format, and the log formatter
  - `src/engine/` — wasm loader and wrappers
  - `src/ai/` — the Web Worker the search runs in, and its controller
  - `src/components/` — board, panels, rules sheet, menu
  - `src/assets/` — the designer's piece and token art, one PNG per side, and
    the rock, which belongs to neither
  - `src/wasm-pkg/` — generated wasm output (gitignored)

## Build & Run Commands

- `pnpm install` — install frontend dependencies
- `pnpm run dev` — Vite dev server
- `pnpm run build:wasm` — rebuild the wasm bindings into `src/wasm-pkg/`
- `pnpm run build` — build wasm, then the production site
- `pnpm run check` — svelte-check
- `pnpm test` — frontend tests (Vitest)
- `cargo test` — Rust tests
- `pnpm run run-games -- <subcommand>` — the runner: `simulate`, `random`,
  `replay`, `bench`

## Architecture Notes

- **Every rule lives in `assedio-core`.** The frontend asks the engine for
  legal choices and never decides anything about the game itself. A rule
  implemented in TypeScript is a bug.
- `GameState` is `Copy`: fixed arrays, no `Vec`, so a determinization is a
  memcpy. Keep it that way — the search clones one per iteration.
- A turn is modelled as separate decisions — slide, activation order, piece
  move, and one target per attacker in an Attack — rather than one compound
  choice, to keep the branching factor down. `settle` runs the game past
  everything that carries no decision, so `Phase::Activate` always has a
  Move-side token at the head of the queue; an Attack side goes to
  `Phase::Attack`, which walks the line and stops on each piece with a target.
- Rocks are pieces with the `NEUTRAL` owner, so they block moves and fall to
  attacks through the same code as everything else. Anything that indexes by
  `owner` — piece art, a player's colour — has to allow for them.
- `apply_with` is generic over an `EventSink` so the search path builds no
  events at all while the UI path gets a full log.
- The wire format between Rust and TypeScript is pinned by
  `assedio-core/tests/wire_format.rs`. Change a serde attribute and change
  `src/data/types.ts` to match; the test is what catches the drift.
- Changes to the evaluation in `search.rs` are judged by
  `pnpm run run-games -- simulate`, playing the new numbers against the old.
  `--params-a`/`--params-b` load weights from JSON.
- `pnpm run run-games -- tune` searches those weights by self-play, via the
  `mcts-tune` crate. `assedio-runner/src/tune.rs` holds the gene mapping;
  it lives in the runner, not the core, because `mcts-tune` uses threads and the
  core compiles to wasm. `scale` is pinned — it is redundant with the weights it
  divides. `--resume` continues from `<output>/checkpoint.json`; pass the same
  `--seed-params`, since fitness is a win rate against those weights.
  `--field round-robin` measures candidates against each other instead of a
  fixed baseline, which is what stops a long run saturating; fitness is then
  relative and cannot be read as progress.

## Testing

- **Rust**: inline `#[cfg(test)]` modules in `rules.rs` plus the wire-format
  integration test. Run with `cargo test`.
- **TypeScript**: Vitest, `src/**/*.test.ts`. Run with `pnpm test`.

## Code Conventions

- Default `rustfmt` for Rust; no explicit formatter config for TypeScript
- Comments are minimal and explain *why*, not what
- Serde JSON is the format across every boundary; structs use
  `rename_all = "camelCase"` and enums additionally `rename_all_fields`
- Svelte 5 runes. Do not name a variable `state` — it shadows the `$state` rune
  and Svelte will read `$state.snapshot(state)` as a store subscription. The
  game state is called `game` throughout.
