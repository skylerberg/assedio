# Assedio

A playable implementation of Assedio — a two-player siege game on a board nine
columns across and nine rows deep — with a browser interface and an opponent
driven by Monte Carlo tree search. The complete rules, including every reading
chosen where the rules sheet is silent, are in [RULES.md](RULES.md).

## Running it

```sh
pnpm install
pnpm run build:wasm   # compiles the Rust engine to src/wasm-pkg/
pnpm run dev
```

`pnpm run build` does both steps and emits a static site into `dist/`.

Requires a Rust toolchain with the `wasm32-unknown-unknown` target, and
[`wasm-pack`](https://drager.github.io/wasm-pack/).

## Deploying

`.github/workflows/deploy.yml` builds the site on every push to `main` and
publishes it to GitHub Pages at `https://skylerberg.github.io/assedio/`.
Nothing about the engine is configured here: `mcts` is a git dependency pinned
in the workspace `Cargo.toml`, so the workflow builds the commit that names.
Vite's `base` is relative, so the built site works from whatever path it is
served under — the project page, a renamed repository, or a local preview.

Enable it under *Settings → Pages* by setting the source to *GitHub Actions*.

## Layout

A Rust workspace and a Svelte frontend:

* `assedio-core/` — the whole game engine: board, rules, turn resolution,
  the `mcts::Game` implementation and the evaluation the search leans on.
* `assedio-wasm/` — `wasm-bindgen` bindings. Every function takes and
  returns JSON, so TypeScript never mirrors a Rust memory layout.
* `assedio-runner/` — CLI for batch simulation, replay and benchmarking.
* `src/` — Svelte 5 + TypeScript frontend.
  * `src/data/` — TypeScript mirror of the wire format, the log formatter, the
    turn-by-turn record of a game, and the save that lets one be resumed.
  * `src/engine/` — the wasm loader and its wrappers.
  * `src/ai/` — the worker the search runs in, and the controller driving it.
  * `src/components/` — board, panels, rules sheet, menu.

Every rule lives in `assedio-core`. The frontend asks the engine for legal
choices and never decides anything about the game itself.

## The AI

The opponent is [`mcts`](https://github.com/skylerberg/monte_carlo), the
`monte_carlo` crate, pinned to a commit in `Cargo.toml` — bump it deliberately
rather than tracking that repository's default branch. The game is deterministic
and perfect information, so most of the trait falls away: `determinize_into` is
a bitwise copy, `Side` is `()`, and `CHOICES_ON_REVISIT` is `SameEverywhere`, so
each node's choices are enumerated once rather than on every visit.

Two things about the integration are worth knowing:

**A turn is searched as three decisions, not one.** Enumerating whole turns —
slide × order × piece × destination — would branch in the hundreds at every
node. Splitting a turn into the decisions a player actually makes keeps each
node down to a few dozen children while leaving the set of reachable positions
identical.

**The leaf evaluation is load-bearing.** Both win conditions sit far outside a
random rollout's reach: random play will not walk a trebuchet onto a hilltop or
push a ram up to a castle wall. Left to an uninformed rollout the search learns
nothing. `search.rs` scores material, castles, a trebuchet's distance from the
nearest hilltop and a ram's from the nearest castle, and squashes the difference
into `[0, 1]` — which is what gives the search a gradient to climb.

Search runs at roughly 600k iterations a second natively at a small budget,
falling below 300k by the half-million iterations of the top difficulty. Every
difficulty but the top one answers inside a second in the browser; the top one
trades a few seconds of thinking for the strength.

## Checking it

```sh
cargo test                                     # engine, wire format, tuner
pnpm test                                      # log, turn records, saves, helpers
pnpm run check                                 # svelte-check

pnpm run run-games bench                     # search throughput
pnpm run run-games random --games 400        # rules terminate, both wins reachable
pnpm run run-games replay --iterations 4000  # one game, printed move by move
pnpm run run-games simulate --games 40 --iterations-a 20000 --iterations-b 3000
pnpm run run-games tune --generations 50     # search the evaluation weights
```

Nothing goes between `run-games` and the subcommand. pnpm passes a `--` on to
the script, where the runner reads it as the end of its options and fails to
find the subcommand.

`cargo test` covers each Attack Zone against the printed diagrams, the turn
and token-flip structure, both win conditions, and the JSON wire format the
browser reads — a rename on the Rust side fails a test rather than breaking the
UI silently at runtime.

`simulate` is how any change to the evaluation gets judged: play the new numbers
against the old and count. The search scales monotonically with its budget —
3k iterations beats 200 in two games of three, and 20k beats 3k in four of
five — which is the signal that it is doing real work. Weights load from a file
with `--params-a` and `--params-b`, which is how a tuning run's output gets
confirmed. `--iterations` sets both sides' search budget; `--iterations-a` and
`--iterations-b` override one side, as above.

## Tuning the weights

`tune` searches the fourteen evaluation weights by self-play, on the `mcts-tune`
crate from the `monte_carlo` repository. Candidates are proposed by CMA-ES (or
`--strategy ga`), each plays a few hundred games against the rest of its
generation, and the win rates drive the search. Every generation writes its
best weights to `tuning/gen-NNN.json` and appends a line to
`tuning/history.jsonl`, and the run's answer lands in `tuning/best.json`.

```sh
pnpm run run-games tune --generations 50
pnpm run run-games tune --generations 50 --seed-params weights.json  # start elsewhere
pnpm run run-games tune --generations 50 --field baseline
```

By default the candidates play each other. The field improves with the
population, so there is nothing to saturate and no fixed opponent to specialise
against, and every game scores two candidates rather than one, so a generation
costs half the games of one measured against a fixed opponent. The trade is that
fitness is relative: the population mean is 0.5 in every generation by
construction, so the fitness column is not a progress bar and `simulate` is how
you see whether the run is actually improving.

`--field baseline` measures every candidate against the weights the run started
from instead. That gives fitness an absolute scale you can read progress from,
and it saturates: a 100-generation run here plateaued at generation 8 and by
generation 43 the whole population sat between 0.88 and 0.94, closer together
than the error on measuring them, so selection was ranking noise.

Three things about it are worth knowing before reading a result.

**Fitness is a win rate, so it is noisy, and that governs the whole design.**
The standard error over `n` games is at worst `sqrt(0.25 / n)` — 2.5 percentage
points at 400 games. Candidates that differ by less than that are ranked by
luck. `--games-per-eval` matters more than `--population`; below about 200 a run
is measuring noise. To push back on it, every candidate in a generation plays
the same seeds and every matchup is played from both seats, so a comparison
carries neither each candidate's own luck nor the first-move advantage.

**`scale` is not tuned.** `evaluate` returns `tanh(difference / scale)` and the
difference is linear in the weights, so multiplying every weight *and* `scale`
by the same constant gives a bit-identical evaluation. Tuning both ends of that
family would spend games wandering a direction that cannot change a game.

**A run is resumable.** Every generation writes `tuning/checkpoint.json`
alongside its parameters, through a temporary file and a rename so that a kill
landing mid-write cannot destroy the state it exists to protect. Continue with
the same command plus `--resume`:

```sh
pnpm run run-games tune --threads 10 --generations 100 --resume
```

Pass the same `--seed-params` the original run used; resuming against different
ones — reaching for the previous run's output is the natural mistake — is
refused rather than allowed to happen quietly. They supply `scale`, which the
run does not tune, and under `--field baseline` fitness is a win rate *against
those weights*, so a different set would make every number after the resume
incomparable with every number before it.

### Validating a result

A run's fitness never says whether its answer beats the weights it started
from. Under the round robin the best is scored against its own generation, and
under `--field baseline` it is a maximum over noisy measurements, so the
luckiest candidate wins ties it would lose on a rerun. Play it against the
starting weights before adopting it:

```sh
pnpm run run-games simulate --games 1000 --params-a tuning/best.json --iterations 4000
```

Without `--params-b` the other side plays the built-in weights, so add
`--params-b weights.json` if the run started from `--seed-params`. Count a draw
as half a win; over 1,000 games the result carries a standard error of about
1.6 points, so 53% is suggestive and 55% is convincing.

Then repeat at the budgets the browser plays at — Squire 4,000, Knight 30,000,
Marshal 120,000 and Warlord 500,000 iterations (`src/ai/aiController.ts`):

```sh
pnpm run run-games simulate --games 1000 --params-a tuning/best.json --iterations 30000
```

Weights matter most when the tree is shallow, so a run at the default
`--eval-iterations 4000` tunes for Squire, and a deeper search can erase the
gain. The cost of a game grows with its budget, so lower `--threads` if the
machine is busy.

To see whether a round-robin run kept improving, play a few of its generations
against the starting weights the same way. Later generations should score
higher; if they do not, the population was cycling, and an earlier
`tuning/gen-NNN.json` may be the better pick.

## A note on the game

Games are short. Under equal search a game reaches a verdict in about eleven
turns, and six in seven end by destroying a side's siege engines rather than its
castles — an attacker takes one target per Attack, so three castles
take at least three turns to bring down, while two siege engines can be picked
off much sooner. The trebuchet is still what both sides play around: castles and
hilltops share columns, so from the centre hilltop it bears on every castle on
the board and takes one a turn. The counterplay is the telegraph — a token is
slid a full turn before it attacks, so an approach can be seen and answered —
and the rocks, which wall every hilltop in on the middle row and cannot be
killed by the siege train that wants to stand there.
