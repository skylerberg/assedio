//! The game's side of the `mcts` contract, plus the evaluation the search
//! leans on.
//!
//! Assedio is deterministic and perfect information, so most of the
//! trait falls away: `determinize_into` is a copy, `Side` is `()`, and no
//! position's choices can differ between two visits. What does need care is the
//! leaf evaluation. Both win conditions sit far outside a random rollout's
//! reach — random play will not walk a trebuchet onto a hilltop or a ram up to
//! a castle wall — so an uninformed rollout returns a draw almost every time
//! and the search learns nothing. The evaluation below supplies the gradient
//! that random play cannot find.

use std::cell::RefCell;
use std::sync::atomic::AtomicBool;

use mcts::rand_core::Rng;
use mcts::{ChoicesOnRevisit, Config, Game, SearchResult, Searcher, Status};
use serde::{Deserialize, Serialize};

use crate::rules::{apply, attack_preview, dominant_order, legal_choices_into};
use crate::types::*;

/// `default` so that a weights file written before a term existed still loads:
/// the terms it does not name keep the built-in value rather than failing to
/// parse, which matters for every run saved under `tuning/`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EvalParams {
    pub castle: f64,
    pub trebuchet: f64,
    pub battering_ram: f64,
    pub swordsman: f64,
    pub flail: f64,
    pub spearman: f64,
    pub archer: f64,
    /// A trebuchet standing on a hilltop is a live siege weapon; off one it is
    /// an expensive spectator.
    pub hilltop: f64,
    /// The first enemy castle a hilltop trebuchet bears on.
    pub castle_threat: f64,
    /// Per enemy castle in range beyond the first. An Attack takes one target
    /// per piece, so the second and third castle a trebuchet covers are
    /// options, not extra damage — they are worth far less than the first.
    pub castle_threat_extra: f64,
    /// Per enemy castle a battering ram currently bears on.
    pub ram_threat: f64,
    /// Per square of distance from a trebuchet to the nearest hilltop.
    pub trebuchet_approach: f64,
    /// Per square of distance from a ram to the nearest standing enemy castle.
    pub ram_approach: f64,
    /// Charged to a side down to its last siege engine, which is one attack
    /// away from being unable to win at all.
    pub last_siege_engine: f64,
    /// Per square an ordinary piece has come forward from its own back row.
    ///
    /// Without this the four pieces that fight for the board are worth the
    /// same number wherever they stand, and staying home costs nothing: the
    /// search has no reason to develop one at all, and an opening turn
    /// evaluates identically whether or not it moves.
    pub advance: f64,
    /// Per ordinary piece loaded in the side's best-armed line: standing in a
    /// line one of its tokens names, with something in its Attack Zone to
    /// take.
    ///
    /// An Attack is issued along a token's line and resolves one piece at a
    /// time, so a piece out of every armed line is a piece that never fires.
    /// This is what loading a line is worth before the Attack is made, and so
    /// what makes sliding a token onto a line — and moving into one — a plan
    /// rather than a coin flip.
    pub threat: f64,
    /// Divides the score difference before it is squashed into `[0, 1]`.
    pub scale: f64,
}

impl Default for EvalParams {
    fn default() -> Self {
        Self {
            castle: 8.504109244173879,
            trebuchet: 6.2137670252547546,
            battering_ram: 11.128680765708523,
            swordsman: 2.514114455552286,
            flail: 2.1676265220376236,
            spearman: 2.188072682231345,
            archer: 3.5427446288138515,
            hilltop: 3.2584214004592957,
            castle_threat: 2.718541543087904,
            castle_threat_extra: 0.5988628571113838,
            ram_threat: 2.0106546877202005,
            trebuchet_approach: 0.00464988260198874,
            ram_approach: 0.0,
            last_siege_engine: 3.4657934555696936,
            advance: 0.15,
            threat: 1.0,
            scale: 14.0,
        }
    }
}

impl EvalParams {
    pub fn piece_value(&self, kind: PieceKind) -> f64 {
        match kind {
            PieceKind::Swordsman => self.swordsman,
            PieceKind::Flail => self.flail,
            PieceKind::Spearman => self.spearman,
            PieceKind::Archer => self.archer,
            PieceKind::Trebuchet => self.trebuchet,
            PieceKind::BatteringRam => self.battering_ram,
            // A rock belongs to neither side, so no side's score ever holds one.
            PieceKind::Rock => 0.0,
        }
    }
}

/// Standing enemy castles a piece of `kind` on `from` would hit right now.
pub fn castles_in_range(state: &GameState, from: Square, enemy: u8, kind: PieceKind) -> usize {
    (kind.zone_mask(from) & state.standing_castles(enemy)).count_ones() as usize
}

/// A destroyed castle becomes a hilltop, so the set a trebuchet is walking
/// towards grows as castles fall.
fn distance_to_hilltop(state: &GameState, from: Square) -> u8 {
    let printed = HILLTOP_COLS
        .iter()
        .map(|&col| from.chebyshev(Square::new(HILLTOP_ROW, col)));
    let fallen = (0..NUM_PLAYERS as u8).flat_map(move |owner| {
        (0..CASTLES_PER_PLAYER)
            .filter(move |&index| !state.castles[owner as usize][index])
            .map(move |index| from.chebyshev(GameState::castle_square(owner, index)))
    });
    printed.chain(fallen).min().expect("the board has hilltops")
}

fn distance_to_enemy_castle(state: &GameState, from: Square, enemy: u8) -> u8 {
    (0..CASTLES_PER_PLAYER)
        .filter(|&index| state.castles[enemy as usize][index])
        .map(|index| from.chebyshev(GameState::castle_square(enemy, index)))
        .min()
        .unwrap_or(0)
}

/// Ordinary pieces the side has loaded: standing in a line one of its tokens
/// names, with a target in their Attack Zone.
///
/// Taken over both tokens and counted on the better of the two, because a line
/// is loaded whether or not it is this turn's to fire — the side holding a
/// loaded line is the side with an Attack in hand. Siege engines are left out:
/// what a trebuchet or a ram bears on is already scored by the castle terms,
/// and counting it here too would pay for the same castle twice.
fn loaded_pieces(state: &GameState, player: u8) -> u32 {
    TOKEN_KINDS
        .iter()
        .map(|&kind| {
            squares_of(attack_preview(state, player, kind).attackers)
                .filter(|&square| {
                    !state
                        .piece_at(square)
                        .expect("an attacker is an occupied square")
                        .kind
                        .is_siege()
                })
                .count() as u32
        })
        .max()
        .unwrap_or(0)
}

/// One side's standing in arbitrary units. Only the difference between the two
/// sides is ever used.
pub fn side_score(state: &GameState, player: u8, params: &EvalParams) -> f64 {
    let enemy = GameState::opponent(player);
    let mut score = params.castle * state.castles_standing(player) as f64;
    let mut siege_engines = 0;

    for (square, piece) in state.pieces_of(player) {
        score += params.piece_value(piece.kind);
        match piece.kind {
            PieceKind::Trebuchet => {
                siege_engines += 1;
                if state.is_hilltop(square) {
                    score += params.hilltop;
                    let threatened = castles_in_range(state, square, enemy, piece.kind);
                    if threatened > 0 {
                        score += params.castle_threat
                            + params.castle_threat_extra * (threatened - 1) as f64;
                    }
                } else {
                    score -= params.trebuchet_approach * distance_to_hilltop(state, square) as f64;
                }
            }
            PieceKind::BatteringRam => {
                siege_engines += 1;
                let threatened = castles_in_range(state, square, enemy, piece.kind);
                if threatened > 0 {
                    score += params.ram_threat * threatened as f64;
                } else {
                    score -=
                        params.ram_approach * distance_to_enemy_castle(state, square, enemy) as f64;
                }
            }
            // The four ordinary pieces have no objective of their own, so
            // apart from this the score says nothing about where they stand.
            PieceKind::Swordsman | PieceKind::Flail | PieceKind::Spearman | PieceKind::Archer => {
                score += params.advance * square.row.abs_diff(BACK_ROW[player as usize]) as f64;
            }
            // A rock belongs to neither side, so it is never in this loop.
            PieceKind::Rock => {}
        }
    }

    if siege_engines == 1 {
        score -= params.last_siege_engine;
    }
    score + params.threat * loaded_pieces(state, player) as f64
}

pub fn terminal_rewards(outcome: Outcome) -> [f64; 2] {
    match outcome {
        Outcome::Winner { player } => {
            let mut rewards = [0.0; 2];
            rewards[player as usize] = 1.0;
            rewards
        }
        Outcome::Draw => [0.5, 0.5],
    }
}

/// Rewards in `[0, 1]`, which is the range `Config::min_reward` and
/// `Config::max_reward` are set to.
pub fn evaluate(state: &GameState, params: &EvalParams) -> [f64; 2] {
    if let Some(outcome) = state.outcome {
        return terminal_rewards(outcome);
    }
    let difference = side_score(state, 0, params) - side_score(state, 1, params);
    let light = 0.5 + 0.5 * (difference / params.scale).tanh();
    [light, 1.0 - light]
}

/// Constant for a whole search. Held out of `GameState` so it is not copied on
/// every determinization.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchContext {
    pub params: EvalParams,
    /// Plies of random play before the leaf is evaluated. Zero evaluates the
    /// leaf directly.
    pub rollout_plies: u32,
}

impl Default for SearchContext {
    fn default() -> Self {
        Self {
            params: EvalParams::default(),
            rollout_plies: 8,
        }
    }
}

thread_local! {
    /// Reused across rollouts so the hot path does not allocate. Nothing
    /// reached from a rollout touches this again, so the borrow cannot nest.
    static ROLLOUT_CHOICES: RefCell<Vec<Choice>> = const { RefCell::new(Vec::new()) };
}

#[inline]
fn uniform_index<R: Rng + ?Sized>(rng: &mut R, len: usize) -> usize {
    ((rng.next_u64() as u128 * len as u128) >> 64) as usize
}

impl Game for GameState {
    type Choice = Choice;
    type Rewards = [f64; 2];
    type Context = SearchContext;
    type Side = ();

    /// True in the strong sense rather than the plausible one:
    /// `determinize_into` is a bitwise copy and nothing is drawn by chance, so
    /// the path to a node fixes its position and the position fixes its choices.
    const CHOICES_ON_REVISIT: ChoicesOnRevisit = ChoicesOnRevisit::SameEverywhere;

    fn status(&self, _ctx: &SearchContext) -> Status<[f64; 2]> {
        match self.outcome {
            Some(outcome) => Status::Terminal(terminal_rewards(outcome)),
            None => Status::Active {
                player: self.current_player,
            },
        }
    }

    fn choices_into(&self, _ctx: &SearchContext, out: &mut Vec<Choice>) {
        match dominant_order(self) {
            Some(first) => {
                out.clear();
                out.push(Choice::Order { first });
            }
            None => legal_choices_into(self, out),
        }
    }

    fn apply_choice<R: Rng + ?Sized>(
        &mut self,
        _ctx: &SearchContext,
        choice: &Choice,
        _rng: &mut R,
    ) {
        apply(self, choice);
    }

    fn rollout<R: Rng + ?Sized>(&mut self, ctx: &SearchContext, rng: &mut R) -> [f64; 2] {
        ROLLOUT_CHOICES.with_borrow_mut(|buf| {
            for _ in 0..ctx.rollout_plies {
                if self.outcome.is_some() {
                    break;
                }
                legal_choices_into(self, buf);
                if buf.is_empty() {
                    break;
                }
                let choice = buf[uniform_index(rng, buf.len())];
                apply(self, &choice);
            }
        });
        evaluate(self, &ctx.params)
    }

    fn new_buffer(&self) -> Self {
        *self
    }

    fn determinize_into<R: Rng + ?Sized>(
        &self,
        dest: &mut Self,
        _ctx: &SearchContext,
        _perspective: u8,
        _rng: &mut R,
    ) {
        *dest = *self;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub iterations: u32,
    /// Only honoured when `assedio-core` is built with the `time`
    /// feature; `wasm32` builds are not, because `Instant` panics there.
    pub time_limit_ms: Option<u64>,
    pub exploration_constant: f64,
    pub context: SearchContext,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            iterations: 50_000,
            time_limit_ms: None,
            exploration_constant: 0.75,
            context: SearchContext::default(),
        }
    }
}

impl AiConfig {
    pub fn with_iterations(iterations: u32) -> Self {
        Self {
            iterations,
            ..Self::default()
        }
    }

    /// The search configuration this budget describes.
    ///
    /// Public so that offline tooling measures the parameters against the same
    /// search the game runs, rather than against a second copy of these numbers
    /// that can drift from it.
    pub fn mcts_config(&self) -> Config {
        Config {
            iterations: self.iterations,
            time_limit_ms: self.time_limit_ms,
            exploration_constant: self.exploration_constant,
            progressive_bias_weight: 0.0,
            early_termination: true,
            max_reward: 1.0,
            min_reward: 0.0,
            ..Config::default()
        }
    }
}

/// A searcher that keeps its tree between moves.
///
/// Worth using from the runner, where one process plays a whole game. The
/// browser rebuilds the state from JSON on every call and so gets no reuse;
/// [`choose_move`] is the entry point there.
pub struct Ai {
    searcher: Searcher<GameState>,
}

impl Ai {
    pub fn new(state: &GameState) -> Self {
        Self {
            searcher: Searcher::new(state),
        }
    }

    pub fn search<R: Rng + ?Sized>(
        &mut self,
        state: &GameState,
        player: u8,
        cfg: &AiConfig,
        cancel: Option<&AtomicBool>,
        rng: &mut R,
    ) -> SearchResult<Choice> {
        self.searcher
            .search(state, &cfg.context, player, &cfg.mcts_config(), cancel, rng)
    }

    /// Keep the subtree under `choice` for the next search.
    pub fn advance(&mut self, choice: &Choice) {
        self.searcher.reuse_subtree(choice);
    }
}

/// One search from scratch. Every decision within a turn — the slide, the
/// activation order, the piece move — is a separate call.
pub fn choose_move<R: Rng + ?Sized>(
    state: &GameState,
    player: u8,
    cfg: &AiConfig,
    rng: &mut R,
) -> Choice {
    Ai::new(state).search(state, player, cfg, None, rng).choice
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand_core::SeedableRng;
    use crate::rules::{initial_state, legal_choices};
    use crate::types::{Board, Piece, Square, NEUTRAL};

    /// The opening with the board swept clean, so a test can state exactly the
    /// position it means. Castles and command tokens are left as dealt.
    fn empty_board() -> GameState {
        let mut state = initial_state();
        state.board = Board::EMPTY;
        state
    }

    fn place(state: &mut GameState, row: u8, col: u8, kind: PieceKind, owner: u8) {
        state.set_piece(Square::new(row, col), Some(Piece { kind, owner }));
    }

    /// Point `player`'s `kind` token at `line`.
    fn aim(state: &mut GameState, player: u8, kind: TokenKind, line: u8) {
        state.token_mut(player, kind).line = line;
    }

    #[test]
    fn the_dominant_order_is_answered_without_a_search() {
        let mut state = initial_state();
        let slide = legal_choices(&state)[0];
        apply(&mut state, &slide);
        let first = dominant_order(&state).expect("the opening Attack has nothing in range");

        let config = AiConfig::with_iterations(1_000);
        let mut rng = wyrand::WyRand::seed_from_u64(1);
        let result = Ai::new(&state).search(&state, state.current_player, &config, None, &mut rng);

        assert_eq!(result.choice, Choice::Order { first });
        assert_eq!(result.iterations_used, 0);
    }

    /// The defect the `advance` term exists for: a piece was worth exactly as
    /// much on its own back row as in the enemy's half, so no opening move
    /// beat standing still.
    #[test]
    fn an_ordinary_piece_is_worth_more_forward_than_at_home() {
        let params = EvalParams::default();
        let mut state = empty_board();
        place(&mut state, 0, 2, PieceKind::Swordsman, 0);
        let home = side_score(&state, 0, &params);

        state.set_piece(Square::new(0, 2), None);
        place(&mut state, 5, 2, PieceKind::Swordsman, 0);
        let forward = side_score(&state, 0, &params);

        assert!(
            forward > home,
            "a swordsman in the enemy's half scored {forward}, no better than one at home at {home}"
        );
    }

    /// And what makes developing worth the turn: a piece that is in an armed
    /// line with something to take is worth more than the same piece one rank
    /// off it.
    #[test]
    fn a_piece_loaded_into_an_armed_line_is_worth_something() {
        let params = EvalParams::default();
        let mut state = empty_board();
        place(&mut state, 4, 2, PieceKind::Swordsman, 0);
        place(&mut state, 4, 3, PieceKind::Flail, 1);
        aim(&mut state, 0, TokenKind::March, 5);
        let off_line = side_score(&state, 0, &params);

        aim(&mut state, 0, TokenKind::March, 4);
        let loaded = side_score(&state, 0, &params);

        assert!(
            loaded > off_line,
            "a swordsman loaded onto the armed row scored {loaded}, no better than {off_line} off it"
        );
    }

    /// A rock is nobody's piece, so it belongs to neither side's score however
    /// far up the board it is — and nothing may be paid for it twice.
    #[test]
    fn the_neutral_rocks_are_worth_nothing_to_either_side() {
        let params = EvalParams::default();
        let mut state = empty_board();
        let bare = (
            side_score(&state, 0, &params),
            side_score(&state, 1, &params),
        );

        place(&mut state, 6, 0, PieceKind::Rock, NEUTRAL);
        assert_eq!(
            bare,
            (
                side_score(&state, 0, &params),
                side_score(&state, 1, &params)
            )
        );
    }
}
