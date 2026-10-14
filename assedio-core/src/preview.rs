//! Read-only questions the interface asks about a position.
//!
//! Every answer is produced by running the real rules forward, so the browser
//! never reasons about the game itself — it only asks "what would happen if"
//! and draws the reply.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::rules::{apply, attack_preview, legal_choices, targets, zone_squares, AttackReach};
use crate::types::*;

/// Positions a single [`forced_order`] enumeration may visit before it gives
/// up. An Attack is a decision per attacker, so a busy line multiplies out
/// fast. Past the budget the interface asks the question rather than guessing
/// at it, which costs the player one click and never a decision.
const ORDER_BUDGET: usize = 30_000;

/// The activation order to take on the player's behalf, or `None` when which
/// one goes first is a decision worth putting to them.
///
/// The player would otherwise be asked to order their activations every turn,
/// and usually there is nothing in the question. An Attack with nothing in
/// range only gives ground by going first: the move it waits for is what can
/// arm it, and any target it then finds may still be spared, so the move leads.
/// Past that the orders are enumerated to the end of the turn, and the question
/// is dropped only when both reach exactly the same positions.
pub fn forced_order(state: &GameState) -> Option<TokenKind> {
    if state.phase != Phase::Order {
        return None;
    }
    let player = state.current_player;
    let armed = TOKEN_KINDS
        .iter()
        .copied()
        .find(|&kind| state.token(player, kind).side == TokenSide::Attack)?;
    if attack_preview(state, player, armed).attackers == 0 {
        return Some(armed.other());
    }

    let mut budget = ORDER_BUDGET;
    let march_first = turn_endings(state, TokenKind::March, &mut budget)?;
    let flank_first = turn_endings(state, TokenKind::Flank, &mut budget)?;
    // Nothing separates the two, so attacking first is the legible one: the
    // Attack is over and done with before the player is asked to move.
    (march_first == flank_first).then_some(armed)
}

/// Every position this turn can end on once `first` activates first, or `None`
/// if the enumeration ran out of budget.
fn turn_endings(
    state: &GameState,
    first: TokenKind,
    budget: &mut usize,
) -> Option<HashSet<GameState>> {
    let mut next = *state;
    apply(&mut next, &Choice::Order { first });
    let mut endings = HashSet::new();
    collect_endings(&next, &mut endings, budget)?;
    Some(endings)
}

fn collect_endings(
    state: &GameState,
    out: &mut HashSet<GameState>,
    budget: &mut usize,
) -> Option<()> {
    *budget = budget.checked_sub(1)?;
    if state.outcome.is_some() || !matches!(state.phase, Phase::Activate | Phase::Attack) {
        out.insert(*state);
        return Some(());
    }
    for choice in legal_choices(state) {
        let mut next = *state;
        apply(&mut next, &choice);
        collect_endings(&next, out, budget)?;
    }
    Some(())
}

/// What one move puts within reach of the Attack still to come this turn.
///
/// Moving a piece onto the line you armed last turn is the game's central
/// combination, and it is the one thing a player cannot read off the board:
/// whether it pays depends on where the piece lands. These are targets rather
/// than casualties — each attacker takes one, and the player picks which.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    pub from: Square,
    pub to: Square,
    pub threatened_pieces: Vec<Square>,
    pub threatened_castles: Vec<Square>,
}

pub fn move_outcomes(state: &GameState) -> Vec<MoveOutcome> {
    if state.phase != Phase::Activate {
        return Vec::new();
    }
    let player = state.current_player;
    legal_choices(state)
        .into_iter()
        .filter_map(|choice| {
            let Choice::Move { from, to } = choice else {
                return None;
            };
            let mut next = *state;
            apply(&mut next, &choice);
            let (threatened_pieces, threatened_castles) = pending_threat(&next, player);
            Some(MoveOutcome {
                from,
                to,
                threatened_pieces,
                threatened_castles,
            })
        })
        .collect()
}

/// What `player`'s Attack still to come bears on. Empty once it has been
/// made, or once the turn has passed on: there is then nothing left to set up.
fn pending_threat(state: &GameState, player: u8) -> (Vec<Square>, Vec<Square>) {
    if state.current_player != player || state.pending_len == 0 {
        return (Vec::new(), Vec::new());
    }
    let kind = state.pending[0];
    if state.token(player, kind).side != TokenSide::Attack {
        return (Vec::new(), Vec::new());
    }
    let reach = attack_preview(state, player, kind);
    (
        AttackReach::squares(reach.pieces).collect(),
        threatened_castles(&reach, GameState::opponent(player)),
    )
}

/// What sliding the token to a line sets up.
///
/// A slide decides two things a turn apart — which pieces may move now, and
/// where the token will attack from next turn — and the second is the one
/// players miss. Both are reported for the line as it stands today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideOutcome {
    pub token: TokenKind,
    pub line: u8,
    /// Pieces on the line that would have a legal move under this token.
    pub movers: Vec<Square>,
    /// The Attack Zone of the line's pieces, occupied today or not.
    pub zone: Vec<Square>,
    /// Pieces, enemies and rocks, standing in that zone right now.
    pub threatened_pieces: Vec<Square>,
    /// Enemy castles that the line's siege engines bear on right now.
    pub threatened_castles: Vec<Square>,
}

pub fn slide_outcomes(state: &GameState) -> Vec<SlideOutcome> {
    if state.phase != Phase::Slide {
        return Vec::new();
    }
    let player = state.current_player;
    let token = state.sliding_token();
    legal_choices(state)
        .into_iter()
        .filter_map(|choice| {
            let Choice::Slide { line } = choice else {
                return None;
            };

            let mut probe = *state;
            probe.token_mut(player, token).line = line;
            let reach = attack_preview(&probe, player, token);

            Some(SlideOutcome {
                token,
                line,
                movers: movers_after(state, &choice, token),
                zone: AttackReach::squares(reach.zone).collect(),
                threatened_pieces: AttackReach::squares(reach.pieces).collect(),
                threatened_castles: threatened_castles(&reach, GameState::opponent(player)),
            })
        })
        .collect()
}

/// The distinct pieces that could move once `slide` is played and `token`
/// activates first. Taking the token first is what the board is showing — an
/// Attack resolved ahead of the move can only free more squares, never fewer.
fn movers_after(state: &GameState, slide: &Choice, token: TokenKind) -> Vec<Square> {
    let mut next = *state;
    apply(&mut next, slide);
    apply(&mut next, &Choice::Order { first: token });
    let mut movers = Vec::new();
    for choice in legal_choices(&next) {
        if let Choice::Move { from, .. } = choice {
            if !movers.contains(&from) {
                movers.push(from);
            }
        }
    }
    movers
}

/// The attackers of the Attack in progress that have still to be asked for a
/// target, the one choosing now first.
///
/// Read as the attackers left if none of them takes anything: a kill that
/// empties a square can leave a later attacker with nothing in reach, and the
/// Attack then skips it.
pub fn pending_attackers(state: &GameState) -> Vec<Square> {
    let mut squares = Vec::new();
    let mut probe = *state;
    while probe.phase == Phase::Attack && probe.outcome.is_none() {
        let Some(choice) = legal_choices(&probe)
            .into_iter()
            .find(|choice| matches!(choice, Choice::HoldFire { .. }))
        else {
            break;
        };
        let Choice::HoldFire { from } = choice else {
            break;
        };
        squares.push(from);
        apply(&mut probe, &choice);
    }
    squares
}

/// One piece's Attack Zone, and what in it the piece could take today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PieceReach {
    /// For a siege engine, only the enemy castles in range: the empty squares
    /// around a ram are not a threat to anything, and drawing them says so.
    pub zone: Vec<Square>,
    pub targets: Vec<Square>,
}

pub fn piece_reach(state: &GameState, square: Square) -> Option<PieceReach> {
    let piece = state.piece_at(square)?;
    let zone = zone_squares(square, piece.kind)
        .filter(|&target| {
            !piece.kind.is_siege()
                || state
                    .standing_castle_at(target)
                    .is_some_and(|owner| owner != piece.owner)
        })
        .collect();
    Some(PieceReach {
        zone,
        targets: targets(state, square, piece).collect(),
    })
}

/// One attacker on an armed line, and everything it could take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    pub from: Square,
    pub kind: PieceKind,
    pub targets: Vec<Square>,
}

/// The pieces an Attack along `kind`'s line would ask for a target, in the
/// order it would ask them. An Attack's reach says what is in danger; this
/// says from whom, which is what a player looking at a threatened piece wants.
pub fn attack_shots(state: &GameState, player: u8, kind: TokenKind) -> Vec<Shot> {
    let line = state.token(player, kind).line;
    GameState::line_squares(kind, line)
        .filter_map(|from| {
            let piece = state.piece_at(from).filter(|piece| piece.owner == player)?;
            let targets: Vec<Square> = targets(state, from, piece).collect();
            (!targets.is_empty()).then_some(Shot {
                from,
                kind: piece.kind,
                targets,
            })
        })
        .collect()
}

fn threatened_castles(reach: &AttackReach, enemy: u8) -> Vec<Square> {
    reach
        .castles
        .iter()
        .enumerate()
        .filter(|(_, &hit)| hit)
        .map(|(index, _)| GameState::castle_square(enemy, index))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{initial_state, settle_state};

    /// The opening position swept clear, rocks included, so a test can state
    /// exactly the position it means. Castles and tokens are left as dealt.
    fn empty_board() -> GameState {
        let mut state = initial_state();
        state.board = [[None; BOARD_COLS]; BOARD_ROWS];
        state
    }

    fn place(state: &mut GameState, row: u8, col: u8, kind: PieceKind, owner: u8) {
        state.set_piece(Square::new(row, col), Some(Piece { kind, owner }));
    }

    /// Both sides keep their siege engines out of the way, on the outer
    /// columns that carry no terrain, so no test position is accidentally
    /// already won and no ram is accidentally beside a castle.
    fn spare_engines(state: &mut GameState) {
        place(state, 5, 0, PieceKind::Trebuchet, 0);
        place(state, 5, 8, PieceKind::BatteringRam, 0);
        place(state, 6, 0, PieceKind::Trebuchet, 1);
        place(state, 6, 8, PieceKind::BatteringRam, 1);
    }

    fn token(state: &mut GameState, player: u8, kind: TokenKind, line: u8, side: TokenSide) {
        *state.token_mut(player, kind) = Token { line, side };
    }

    /// Mid-turn, slide already made, both activations still to come.
    fn order_phase(state: &mut GameState, player: u8) {
        state.current_player = player;
        state.pending = [TokenKind::March, TokenKind::Flank];
        state.pending_len = 2;
        state.phase = Phase::Order;
    }

    /// Mid-turn with `first` at the head of the queue and the other behind it.
    fn activate_phase(state: &mut GameState, player: u8, first: TokenKind) {
        state.current_player = player;
        state.pending = [first, first.other()];
        state.pending_len = 2;
        state.phase = Phase::Activate;
    }

    /// A swordsman that can leave the attack row but has nothing to take from
    /// it, so the Attack is idle until the move says otherwise.
    fn barren_attack() -> GameState {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 0, 1, PieceKind::Swordsman, 0);
        token(&mut state, 0, TokenKind::March, 0, TokenSide::Attack);
        token(&mut state, 0, TokenKind::Flank, 1, TokenSide::Move);
        order_phase(&mut state, 0);
        state
    }

    #[test]
    fn the_move_leads_when_the_attack_has_nothing_to_take() {
        assert_eq!(forced_order(&barren_attack()), Some(TokenKind::Flank));
    }

    #[test]
    fn the_move_leads_even_when_it_is_what_arms_the_attack() {
        // The row the March token is armed on is empty of ours, and the Flank
        // token can walk the swordsman on b1 onto it beside the enemy on d2.
        // Attacking first would throw that away, so the player is not offered
        // it.
        let mut state = barren_attack();
        token(&mut state, 0, TokenKind::March, 1, TokenSide::Attack);
        place(&mut state, 1, 3, PieceKind::Swordsman, 1);

        assert_eq!(forced_order(&state), Some(TokenKind::Flank));
    }

    #[test]
    fn order_is_a_real_choice_when_moving_away_spares_a_victim() {
        let mut state = barren_attack();
        // The swordsman on b1 now bears on b2 — and stepping off the row to
        // reach b2's diagonal is exactly what spares the piece standing there.
        place(&mut state, 1, 1, PieceKind::Swordsman, 1);

        assert_eq!(forced_order(&state), None);
    }

    #[test]
    fn the_attack_leads_when_the_move_line_is_empty() {
        // The Flank token names a column with nothing of ours on it, so that
        // activation can only pass and the Attack is all there is to sequence.
        let mut state = barren_attack();
        place(&mut state, 1, 1, PieceKind::Swordsman, 1);
        token(&mut state, 0, TokenKind::Flank, 6, TokenSide::Move);

        assert_eq!(forced_order(&state), Some(TokenKind::March));
    }

    #[test]
    fn the_zone_includes_squares_no_one_is_standing_on() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 2, 4, PieceKind::Swordsman, 0);
        token(&mut state, 0, TokenKind::March, 2, TokenSide::Attack);

        let reach = attack_preview(&state, 0, TokenKind::March);
        let zone: Vec<Square> = AttackReach::squares(reach.zone).collect();

        assert_eq!(
            zone,
            vec![
                Square::new(1, 4),
                Square::new(2, 3),
                Square::new(2, 5),
                Square::new(3, 4),
            ],
        );
        // Nothing is standing in it, so the Attack has nothing to take.
        assert_eq!(reach.pieces, 0);
        assert_eq!(reach.attackers, 0);
    }

    #[test]
    fn a_siege_engine_adds_no_square_a_piece_could_stand_on() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 1, 3, PieceKind::BatteringRam, 0);
        token(&mut state, 0, TokenKind::March, 1, TokenSide::Attack);

        assert_eq!(attack_preview(&state, 0, TokenKind::March).zone, 0);
    }

    #[test]
    fn move_outcomes_report_what_each_destination_puts_in_reach() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 0, 1, PieceKind::Swordsman, 0);
        place(&mut state, 1, 3, PieceKind::Swordsman, 1);
        token(&mut state, 0, TokenKind::March, 1, TokenSide::Attack);
        token(&mut state, 0, TokenKind::Flank, 1, TokenSide::Move);
        activate_phase(&mut state, 0, TokenKind::Flank);

        let outcomes = move_outcomes(&state);
        let threatens = |to: Square| {
            outcomes
                .iter()
                .find(|outcome| outcome.to == to)
                .unwrap_or_else(|| panic!("no move to {to:?}"))
                .threatened_pieces
                .clone()
        };

        // Landing on c2 puts the swordsman beside d2 and on the armed row.
        assert_eq!(threatens(Square::new(1, 2)), vec![Square::new(1, 3)]);
        // a2 is on the same row but two squares short of anything.
        assert_eq!(threatens(Square::new(1, 0)), Vec::<Square>::new());
        // Leaving the row altogether disarms the Attack entirely.
        assert_eq!(threatens(Square::new(2, 3)), Vec::<Square>::new());
    }

    #[test]
    fn move_outcomes_are_empty_once_the_attack_is_made() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 0, 1, PieceKind::Swordsman, 0);
        place(&mut state, 1, 3, PieceKind::Swordsman, 1);
        token(&mut state, 0, TokenKind::March, 1, TokenSide::Attack);
        token(&mut state, 0, TokenKind::Flank, 1, TokenSide::Move);
        state.current_player = 0;
        state.pending = [TokenKind::Flank, TokenKind::March];
        state.pending_len = 1;
        state.phase = Phase::Activate;

        assert!(move_outcomes(&state)
            .iter()
            .all(|outcome| outcome.threatened_pieces.is_empty()));
    }

    #[test]
    fn slide_outcomes_name_the_movers_and_the_line_they_arm() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 2, 2, PieceKind::Swordsman, 0);
        place(&mut state, 2, 3, PieceKind::Swordsman, 1);
        token(&mut state, 0, TokenKind::March, 0, TokenSide::Move);
        token(&mut state, 0, TokenKind::Flank, 6, TokenSide::Attack);
        state.current_player = 0;
        state.phase = Phase::Slide;

        let outcomes = slide_outcomes(&state);
        assert!(outcomes
            .iter()
            .all(|outcome| outcome.token == TokenKind::March));
        let at = |line: u8| {
            outcomes
                .iter()
                .find(|outcome| outcome.line == line)
                .unwrap_or_else(|| panic!("no slide to row {line}"))
        };

        let row = at(2);
        assert_eq!(row.movers, vec![Square::new(2, 2)]);
        assert!(row.zone.contains(&Square::new(2, 3)));
        assert_eq!(row.threatened_pieces, vec![Square::new(2, 3)]);

        // The siege row arms nothing a piece could be standing on, and the
        // ram is out of reach of any castle.
        let siege = at(5);
        assert_eq!(siege.zone, Vec::<Square>::new());
        assert_eq!(siege.threatened_castles, Vec::<Square>::new());
        assert_eq!(siege.movers, vec![Square::new(5, 0), Square::new(5, 8)]);

        // A row with nothing of ours on it is a legal slide that frees nothing
        // to move now and arms nothing yet.
        let empty = at(4);
        assert_eq!(empty.movers, Vec::<Square>::new());
        assert_eq!(empty.zone, Vec::<Square>::new());
    }

    #[test]
    fn pending_attackers_lists_the_attackers_still_to_choose() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 2, 1, PieceKind::Swordsman, 0);
        place(&mut state, 2, 4, PieceKind::Swordsman, 0);
        place(&mut state, 2, 2, PieceKind::Swordsman, 1);
        place(&mut state, 2, 5, PieceKind::Swordsman, 1);
        token(&mut state, 0, TokenKind::March, 2, TokenSide::Attack);
        state.current_player = 0;
        state.pending = [TokenKind::March, TokenKind::Flank];
        state.pending_len = 1;
        state.phase = Phase::Activate;
        settle_state(&mut state);

        assert_eq!(state.phase, Phase::Attack);
        assert_eq!(
            pending_attackers(&state),
            vec![Square::new(2, 1), Square::new(2, 4)],
        );

        // Once the first has taken its target, only the second is still to be
        // asked.
        let attack = legal_choices(&state)
            .into_iter()
            .find(|choice| matches!(choice, Choice::Attack { .. }))
            .expect("the first attacker has a target");
        apply(&mut state, &attack);
        assert_eq!(pending_attackers(&state), vec![Square::new(2, 4)]);
    }

    /// An order taken for the player must cost them nothing: it has to reach
    /// every position the order it replaces could have reached, and the
    /// question may only be dropped when there is one. Random play is where
    /// that claim is cheap to keep honest.
    #[test]
    fn a_taken_order_gives_up_nothing_the_other_could_have_reached() {
        use crate::rand_core::{Rng, SeedableRng};

        let mut rng = wyrand::WyRand::seed_from_u64(11);
        let mut compared = 0;

        for _ in 0..8 {
            let mut state = initial_state();
            while state.outcome.is_none() {
                if state.phase == Phase::Order {
                    let mut budget = 400_000;
                    let march = turn_endings(&state, TokenKind::March, &mut budget);
                    let flank = turn_endings(&state, TokenKind::Flank, &mut budget);
                    if let (Some(march), Some(flank)) = (march, flank) {
                        match forced_order(&state) {
                            Some(TokenKind::March) => assert!(flank.is_subset(&march)),
                            Some(TokenKind::Flank) => assert!(march.is_subset(&flank)),
                            None => {
                                assert_ne!(march, flank, "a settled order was put to the player")
                            }
                        }
                        compared += 1;
                    }
                }
                let choices = legal_choices(&state);
                let index = ((rng.next_u64() as u128 * choices.len() as u128) >> 64) as usize;
                apply(&mut state, &choices[index]);
            }
        }

        assert!(compared > 100, "only {compared} orders were checked");
    }

    #[test]
    fn nothing_is_pending_outside_an_attack() {
        assert_eq!(pending_attackers(&initial_state()), Vec::<Square>::new());
    }

    fn sorted(mut squares: Vec<Square>) -> Vec<Square> {
        squares.sort();
        squares
    }

    #[test]
    fn a_ram_reaches_only_the_enemy_castle_beside_it() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 1, 4, PieceKind::BatteringRam, 1);
        place(&mut state, 1, 3, PieceKind::Swordsman, 0);

        let reach = piece_reach(&state, Square::new(1, 4)).unwrap();

        assert_eq!(reach.zone, vec![Square::new(0, 4)]);
        assert_eq!(reach.targets, vec![Square::new(0, 4)]);
    }

    #[test]
    fn a_swordsman_zone_covers_empty_squares_but_takes_only_enemies_and_rocks() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 2, 4, PieceKind::Swordsman, 0);
        place(&mut state, 2, 5, PieceKind::Flail, 1);
        place(&mut state, 2, 3, PieceKind::Flail, 0);
        place(&mut state, 3, 4, PieceKind::Rock, NEUTRAL);

        let reach = piece_reach(&state, Square::new(2, 4)).unwrap();

        assert_eq!(
            sorted(reach.zone),
            sorted(vec![
                Square::new(1, 4),
                Square::new(3, 4),
                Square::new(2, 3),
                Square::new(2, 5),
            ]),
        );
        assert_eq!(
            sorted(reach.targets),
            sorted(vec![Square::new(3, 4), Square::new(2, 5)]),
        );
        assert_eq!(piece_reach(&state, Square::new(4, 4)), None);
    }

    #[test]
    fn shots_name_each_attacker_with_a_target_in_line_order() {
        let mut state = empty_board();
        spare_engines(&mut state);
        place(&mut state, 2, 1, PieceKind::Swordsman, 0);
        place(&mut state, 3, 1, PieceKind::Flail, 1);
        place(&mut state, 2, 4, PieceKind::Archer, 0);
        place(&mut state, 2, 6, PieceKind::Spearman, 0);
        place(&mut state, 2, 8, PieceKind::Swordsman, 1);
        token(&mut state, 0, TokenKind::March, 2, TokenSide::Attack);

        assert_eq!(
            attack_shots(&state, 0, TokenKind::March),
            vec![
                Shot {
                    from: Square::new(2, 1),
                    kind: PieceKind::Swordsman,
                    targets: vec![Square::new(3, 1)],
                },
                Shot {
                    from: Square::new(2, 6),
                    kind: PieceKind::Spearman,
                    targets: vec![Square::new(2, 8)],
                },
            ],
        );
    }
}
