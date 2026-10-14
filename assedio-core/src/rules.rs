//! Setup, legal-choice generation, and turn resolution.
//!
//! A turn is broken into the decisions a player actually makes — slide, order,
//! move, and a target per attacker — rather than handed to the search as one
//! compound action. Enumerating whole turns would mean a branching factor in
//! the hundreds at every node; this way each node branches by at most a few
//! dozen and the search still sees the same set of reachable positions.

use crate::types::*;

/// Directions a March moves a piece: vertically or horizontally.
const MARCH_DIRS: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
/// Directions a Flank moves a piece: diagonally.
const FLANK_DIRS: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];

/// What one attacker took. Siege engines only ever destroy castles, and every
/// other piece only ever kills a piece — an enemy or a rock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Casualty {
    Piece { piece: Piece },
    Castle,
}

/// What a piece did, for the UI log and for replays. The search never builds
/// one: `apply` takes a sink that compiles away to nothing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameEvent {
    Slid {
        player: u8,
        token: TokenKind,
        from: u8,
        to: u8,
    },
    Moved {
        player: u8,
        token: TokenKind,
        kind: PieceKind,
        from: Square,
        to: Square,
    },
    /// A Move side spent without moving.
    Passed {
        player: u8,
        token: TokenKind,
    },
    /// An Attack side activating along its line. Every attacker in it follows
    /// as its own event, because every attacker is its own decision.
    Attacked {
        player: u8,
        token: TokenKind,
        line: u8,
    },
    /// One attacker taking its one target.
    Struck {
        player: u8,
        token: TokenKind,
        kind: PieceKind,
        from: Square,
        target: Square,
        casualty: Casualty,
    },
    /// An attacker that had a target and did not take it.
    HeldFire {
        player: u8,
        token: TokenKind,
        kind: PieceKind,
        from: Square,
    },
    TurnEnded {
        player: u8,
    },
    GameOver {
        outcome: Outcome,
    },
}

/// Somewhere for `apply` to put events. `()` discards them without ever
/// building one, which is what keeps the search allocation-free.
pub trait EventSink {
    fn emit(&mut self, make: impl FnOnce() -> GameEvent);
}

impl EventSink for () {
    fn emit(&mut self, _make: impl FnOnce() -> GameEvent) {}
}

impl EventSink for Vec<GameEvent> {
    fn emit(&mut self, make: impl FnOnce() -> GameEvent) {
        self.push(make());
    }
}

pub fn initial_state() -> GameState {
    let mut board = [[None; BOARD_COLS]; BOARD_ROWS];

    // Back row: spear, bow, trebuchet, bow, spear across the middle five
    // columns. The castle columns either side of them start empty; the
    // trebuchet starts on top of the middle castle. The outermost column on
    // each wing is empty ground.
    let back = [
        PieceKind::Spearman,
        PieceKind::Archer,
        PieceKind::Trebuchet,
        PieceKind::Archer,
        PieceKind::Spearman,
    ];
    let front = [
        PieceKind::Swordsman,
        PieceKind::Flail,
        PieceKind::BatteringRam,
        PieceKind::Flail,
        PieceKind::Swordsman,
    ];

    for player in 0..NUM_PLAYERS as u8 {
        let back_row = BACK_ROW[player as usize];
        let front_row = if player == 0 {
            back_row + 1
        } else {
            back_row - 1
        };
        for (i, (&back_kind, &front_kind)) in back.iter().zip(front.iter()).enumerate() {
            // The five pieces sit between the outer two castles.
            let col = CASTLE_COLS[0] + 1 + i as u8;
            board[back_row as usize][col as usize] = Some(Piece {
                kind: back_kind,
                owner: player,
            });
            board[front_row as usize][col as usize] = Some(Piece {
                kind: front_kind,
                owner: player,
            });
        }
    }

    for col in ROCK_COLS {
        board[HILLTOP_ROW as usize][col as usize] = Some(Piece {
            kind: PieceKind::Rock,
            owner: NEUTRAL,
        });
    }

    // Both sides open with the March token Attack side up on the middle row and
    // the Flank token Move side up on the middle column, so the first turn is
    // played like every other.
    let opening = [
        Token {
            line: BOARD_ROWS as u8 / 2,
            side: TokenSide::Attack,
        },
        Token {
            line: BOARD_COLS as u8 / 2,
            side: TokenSide::Move,
        },
    ];

    GameState {
        board,
        castles: [[true; CASTLES_PER_PLAYER]; NUM_PLAYERS],
        tokens: [opening; NUM_PLAYERS],
        current_player: 0,
        phase: Phase::Slide,
        pending: [TokenKind::March, TokenKind::Flank],
        pending_len: 0,
        attack_index: 0,
        turn: 0,
        outcome: None,
    }
}

/// Destinations for a piece under `kind`'s Move side: vertical and horizontal
/// lines for a March, diagonals for a Flank. Moving never captures, so a piece
/// stops before any occupied square, rocks included, and a standing enemy
/// castle stops it too: it may neither enter one nor pass through it. Its own
/// castles are open ground — the trebuchet opens the game standing on one.
fn move_destinations_into(
    state: &GameState,
    player: u8,
    from: Square,
    kind: TokenKind,
    out: &mut Vec<Choice>,
) {
    let dirs = match kind {
        TokenKind::March => &MARCH_DIRS,
        TokenKind::Flank => &FLANK_DIRS,
    };
    for &(drow, dcol) in dirs {
        let mut square = from;
        while let Some(next) = square.offset(drow, dcol) {
            if state.piece_at(next).is_some() || state.blocks_move(next, player) {
                break;
            }
            out.push(Choice::Move { from, to: next });
            square = next;
        }
    }
}

fn move_choices_into(state: &GameState, kind: TokenKind, out: &mut Vec<Choice>) {
    let player = state.current_player;
    let line = state.token(player, kind).line;
    for square in GameState::line_squares(kind, line) {
        match state.piece_at(square) {
            Some(piece) if piece.owner == player => {
                move_destinations_into(state, player, square, kind, out)
            }
            _ => {}
        }
    }
}

/// Every square the piece on `from` could take as the board stands: a standing
/// enemy castle for a siege engine, and for any other piece a piece that is
/// not its own — an enemy or a rock.
///
/// The trebuchet's hilltop needs no check of its own. Every square three from a
/// castle is a hilltop or another castle, and an enemy castle can only be stood
/// on once it is rubble, which is a hilltop.
pub(crate) fn targets(
    state: &GameState,
    from: Square,
    piece: Piece,
) -> impl Iterator<Item = Square> + '_ {
    zone_squares(from, piece.kind).filter(move |&target| {
        if piece.kind.is_siege() {
            state
                .standing_castle_at(target)
                .is_some_and(|owner| owner != piece.owner)
        } else {
            state
                .piece_at(target)
                .is_some_and(|victim| victim.owner != piece.owner)
        }
    })
}

/// Every on-board square in a piece's Attack Zone, whoever is standing there.
/// [`targets`] narrows this to the squares holding something to take; the board
/// wants the wider set to show where a move would not survive.
pub(crate) fn zone_squares(from: Square, kind: PieceKind) -> impl Iterator<Item = Square> {
    kind.attack_zone()
        .iter()
        .filter_map(move |&(drow, dcol)| from.offset(drow, dcol))
}

/// The next piece of the current player's on `kind`'s line, at or after index
/// `from`, with a target to take. Pieces with nothing to take are skipped:
/// they carry no decision, so the Attack never stops on them.
fn next_attacker(state: &GameState, kind: TokenKind, from: u8) -> Option<u8> {
    let player = state.current_player;
    let line = state.token(player, kind).line;
    (from..GameState::line_len(kind)).find(|&index| {
        let square = GameState::line_square(kind, line, index);
        state.piece_at(square).is_some_and(|piece| {
            piece.owner == player && targets(state, square, piece).next().is_some()
        })
    })
}

/// The activation order that is never worse than the other, when the position
/// alone shows one. An Attack with nothing in range can only gain by going
/// second: the move it waits for may give it a target, and any target may
/// still be spared, so every position attacking first reaches, moving first
/// reaches too.
pub fn dominant_order(state: &GameState) -> Option<TokenKind> {
    if state.phase != Phase::Order {
        return None;
    }
    let mover = state.sliding_token();
    let attack_is_idle = next_attacker(state, mover.other(), 0).is_none();
    attack_is_idle.then_some(mover)
}

/// The attacker now choosing a target, and the piece standing on it.
fn current_attacker(state: &GameState) -> (Square, Piece) {
    let kind = state.pending[0];
    let line = state.token(state.current_player, kind).line;
    let square = GameState::line_square(kind, line, state.attack_index);
    let piece = state
        .piece_at(square)
        .expect("an Attack only ever stops on a piece with a target");
    (square, piece)
}

pub fn legal_choices_into(state: &GameState, out: &mut Vec<Choice>) {
    out.clear();
    match state.phase {
        Phase::Slide => {
            let kind = state.sliding_token();
            let current = state.token(state.current_player, kind).line;
            out.extend(
                (0..GameState::line_count(kind))
                    .filter(|&line| line != current)
                    .map(|line| Choice::Slide { line }),
            );
        }
        Phase::Order => {
            out.push(Choice::Order {
                first: TokenKind::March,
            });
            out.push(Choice::Order {
                first: TokenKind::Flank,
            });
        }
        Phase::Activate => {
            // `settle` takes every Attack side into `Phase::Attack`, so the
            // head of the queue always shows a Move side here.
            let kind = state.pending[0];
            move_choices_into(state, kind, out);
            out.push(Choice::Pass);
        }
        Phase::Attack => {
            let (from, piece) = current_attacker(state);
            for target in targets(state, from, piece) {
                out.push(Choice::Attack { from, target });
            }
            out.push(Choice::HoldFire { from });
        }
        Phase::GameOver => {}
    }
}

pub fn legal_choices(state: &GameState) -> Vec<Choice> {
    let mut out = Vec::new();
    legal_choices_into(state, &mut out);
    out
}

/// Everything one Attack could take from its line. Each attacker takes at most
/// one of these, so this is the Attack's reach and not its casualty list —
/// which is exactly what a threat overlay wants to show.
#[derive(Default)]
pub struct AttackReach {
    /// Friendly pieces in the line with a target to take, as a square bitboard.
    pub attackers: u64,
    /// Every square in the Attack Zone of this line's pieces, occupied or not.
    /// An attacker with nothing to take today is still counted: walking into
    /// its zone is what gives it a target. Siege engines add nothing — they
    /// destroy castles and leave what stands on one alone.
    pub zone: u64,
    /// Pieces under threat, enemies and rocks alike, as a square bitboard.
    pub pieces: u64,
    /// Enemy castles under threat, indexed by slot.
    pub castles: [bool; CASTLES_PER_PLAYER],
}

impl AttackReach {
    pub fn squares(bits: u64) -> impl Iterator<Item = Square> {
        GameState::squares().filter(move |square| bits >> square.index() & 1 == 1)
    }

    pub fn is_empty(&self) -> bool {
        self.pieces == 0 && !self.castles.iter().any(|&c| c)
    }
}

/// What an Attack with `player`'s `kind` token, at its current line, bears on.
/// Pure: the UI calls it to preview a threat.
pub fn attack_preview(state: &GameState, player: u8, kind: TokenKind) -> AttackReach {
    let line = state.token(player, kind).line;
    let mut reach = AttackReach::default();

    for square in GameState::line_squares(kind, line) {
        let Some(piece) = state.piece_at(square) else {
            continue;
        };
        if piece.owner != player {
            continue;
        }
        if !piece.kind.is_siege() {
            for target in zone_squares(square, piece.kind) {
                reach.zone |= 1 << target.index();
            }
        }
        for target in targets(state, square, piece) {
            reach.attackers |= 1 << square.index();
            if piece.kind.is_siege() {
                let (_, index) =
                    GameState::castle_slot_at(target).expect("a siege engine takes castles");
                reach.castles[index] = true;
            } else {
                reach.pieces |= 1 << target.index();
            }
        }
    }

    reach
}

/// Resolve one attacker. Targets are taken as they are chosen rather than all
/// at once, so an attacker later down the line cannot spend itself on a target
/// an earlier one has already taken.
fn fire(state: &mut GameState, from: Square, target: Square, sink: &mut impl EventSink) {
    let player = state.current_player;
    let token = state.pending[0];
    let piece = state
        .piece_at(from)
        .expect("attacked with a piece that is not there");

    let casualty = if piece.kind.is_siege() {
        let (owner, index) =
            GameState::castle_slot_at(target).expect("a siege engine takes castles");
        state.castles[owner as usize][index] = false;
        Casualty::Castle
    } else {
        let victim = state.piece_at(target).expect("attacked an empty square");
        state.set_piece(target, None);
        Casualty::Piece { piece: victim }
    };

    sink.emit(|| GameEvent::Struck {
        player,
        token,
        kind: piece.kind,
        from,
        target,
        casualty,
    });
}

/// Both win conditions, checked against both players. Only the side that was
/// just attacked can have lost — attacks never touch the attacker's own
/// material — but checking both keeps the rule in one place.
fn check_outcome(state: &mut GameState) {
    if state.outcome.is_some() {
        return;
    }
    for player in 0..NUM_PLAYERS as u8 {
        let destroyed = state.castles_standing(player) == 0;
        let disarmed = state.siege_engines(player) == 0;
        if destroyed || disarmed {
            state.outcome = Some(Outcome::Winner {
                player: GameState::opponent(player),
            });
            return;
        }
    }
}

fn pop_pending(state: &mut GameState) {
    state.pending[0] = state.pending[1];
    state.pending_len -= 1;
    state.attack_index = 0;
}

fn end_turn(state: &mut GameState, sink: &mut impl EventSink) {
    let player = state.current_player;
    sink.emit(|| GameEvent::TurnEnded { player });
    state.current_player = GameState::opponent(player);
    state.pending_len = 0;
    // Reset rather than leave the drained queue and the Attack's cursor
    // behind: two turns that reached the same board by different activation
    // orders are the same position, and stale fields would say otherwise.
    state.pending = [TokenKind::March, TokenKind::Flank];
    state.attack_index = 0;
    state.phase = Phase::Slide;
    if state.current_player == 0 {
        state.turn += 1;
        if state.turn >= MAX_TURNS {
            state.outcome = Some(Outcome::Draw);
        }
    }
}

/// Run the game forward past everything that carries no decision, so that
/// `status` and `legal_choices` are only ever asked about a real choice.
fn settle(state: &mut GameState, sink: &mut impl EventSink) {
    loop {
        if let Some(outcome) = state.outcome {
            if state.phase != Phase::GameOver {
                state.phase = Phase::GameOver;
                sink.emit(|| GameEvent::GameOver { outcome });
            }
            return;
        }
        if !matches!(state.phase, Phase::Activate | Phase::Attack) {
            return;
        }
        if state.pending_len == 0 {
            end_turn(state, sink);
            continue;
        }
        let player = state.current_player;
        let kind = state.pending[0];
        if state.token(player, kind).side == TokenSide::Move {
            state.phase = Phase::Activate;
            return;
        }

        // An Attack side walks its line, stopping on each piece with a target
        // so that piece can pick its single one. Entering the Attack announces
        // it once; the attackers themselves are separate decisions.
        if state.phase != Phase::Attack {
            let line = state.token(player, kind).line;
            sink.emit(|| GameEvent::Attacked {
                player,
                token: kind,
                line,
            });
            state.phase = Phase::Attack;
            state.attack_index = 0;
        }

        match next_attacker(state, kind, state.attack_index) {
            Some(index) => {
                state.attack_index = index;
                return;
            }
            None => {
                let token = state.token_mut(player, kind);
                token.side = token.side.flipped();
                pop_pending(state);
                state.phase = Phase::Activate;
            }
        }
    }
}

/// Play `choice` and run on to the next real decision.
///
/// Panics in debug on an illegal choice; in release an illegal choice quietly
/// produces a nonsense position, so callers enumerate first.
pub fn apply_with<S: EventSink>(state: &mut GameState, choice: &Choice, sink: &mut S) {
    debug_assert!(
        legal_choices(state).contains(choice),
        "assedio: {choice:?} is not legal in {:?}",
        state.phase
    );
    let player = state.current_player;

    match *choice {
        Choice::Slide { line } => {
            let token = state.sliding_token();
            let from = state.token(player, token).line;
            state.token_mut(player, token).line = line;
            sink.emit(|| GameEvent::Slid {
                player,
                token,
                from,
                to: line,
            });
            state.pending = [token, token.other()];
            state.pending_len = 2;
            state.phase = Phase::Order;
        }
        Choice::Order { first } => {
            state.pending = [first, first.other()];
            state.pending_len = 2;
            state.phase = Phase::Activate;
        }
        Choice::Move { from, to } => {
            let token = state.pending[0];
            let piece = state
                .piece_at(from)
                .expect("moved a piece that is not there");
            state.set_piece(from, None);
            state.set_piece(to, Some(piece));
            sink.emit(|| GameEvent::Moved {
                player,
                token,
                kind: piece.kind,
                from,
                to,
            });
            let slot = state.token_mut(player, token);
            slot.side = slot.side.flipped();
            pop_pending(state);
        }
        Choice::Pass => {
            let token = state.pending[0];
            sink.emit(|| GameEvent::Passed { player, token });
            let slot = state.token_mut(player, token);
            slot.side = slot.side.flipped();
            pop_pending(state);
        }
        Choice::Attack { from, target } => {
            fire(state, from, target, sink);
            state.attack_index += 1;
            check_outcome(state);
        }
        Choice::HoldFire { from } => {
            let token = state.pending[0];
            let kind = state
                .piece_at(from)
                .expect("held fire with a piece that is not there")
                .kind;
            sink.emit(|| GameEvent::HeldFire {
                player,
                token,
                kind,
                from,
            });
            state.attack_index += 1;
        }
    }

    settle(state, sink);
}

pub fn apply(state: &mut GameState, choice: &Choice) {
    apply_with(state, choice, &mut ());
}

/// Run a state forward past everything that carries no decision.
///
/// States produced by [`apply`] are already settled. A state assembled by hand
/// — a test position, a puzzle, a saved position from an older build — is not,
/// and must be settled before it is asked for a legal choice: `Phase::Activate`
/// assumes the head of the queue shows a Move side, because the Attack that
/// would otherwise be there has already been resolved.
pub fn settle_state(state: &mut GameState) -> Vec<GameEvent> {
    let mut events = Vec::new();
    settle(state, &mut events);
    events
}

pub fn apply_logged(state: &mut GameState, choice: &Choice) -> Vec<GameEvent> {
    let mut events = Vec::new();
    apply_with(state, choice, &mut events);
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The opening position with every piece swept off, rocks included, so a
    /// test can state exactly the position it means. Castles and tokens are
    /// left as dealt.
    fn empty_board() -> GameState {
        let mut state = initial_state();
        state.board = [[None; BOARD_COLS]; BOARD_ROWS];
        state
    }

    fn place(state: &mut GameState, row: u8, col: u8, kind: PieceKind, owner: u8) {
        state.set_piece(Square::new(row, col), Some(Piece { kind, owner }));
    }

    fn place_rock(state: &mut GameState, row: u8, col: u8) {
        place(state, row, col, PieceKind::Rock, NEUTRAL);
    }

    /// Mid-turn, with `kind` the only activation left and showing its Move
    /// side on `line`.
    fn move_with(state: &mut GameState, kind: TokenKind, line: u8) {
        state.current_player = 0;
        state.phase = Phase::Activate;
        state.pending = [kind, kind.other()];
        state.pending_len = 1;
        *state.token_mut(0, kind) = Token {
            line,
            side: TokenSide::Move,
        };
    }

    fn destinations(state: &GameState) -> Vec<Square> {
        legal_choices(state)
            .into_iter()
            .filter_map(|choice| match choice {
                Choice::Move { to, .. } => Some(to),
                _ => None,
            })
            .collect()
    }

    /// Point `player`'s token at `line` with its Attack side up, and queue it
    /// as the only activation left this turn.
    fn arm_attack(state: &mut GameState, player: u8, kind: TokenKind, line: u8) {
        state.current_player = player;
        *state.token_mut(player, kind) = Token {
            line,
            side: TokenSide::Attack,
        };
        state.pending = [kind, kind.other()];
        state.pending_len = 1;
        state.phase = Phase::Activate;
    }

    /// The pieces and castles an Attack with `kind` bears on. Each attacker
    /// takes one target, so this is the Attack's reach, not its body count.
    fn threatened_by(state: &GameState, player: u8, kind: TokenKind) -> (Vec<Square>, Vec<Square>) {
        let reach = attack_preview(state, player, kind);
        let enemy = GameState::opponent(player);
        let pieces = AttackReach::squares(reach.pieces).collect();
        let castles = reach
            .castles
            .iter()
            .enumerate()
            .filter(|(_, &hit)| hit)
            .map(|(index, _)| GameState::castle_square(enemy, index))
            .collect();
        (pieces, castles)
    }

    /// Play a whole Attack out, taking the first target offered at every stop.
    fn attack_at_will(state: &mut GameState) {
        while state.phase == Phase::Attack {
            let attack = legal_choices(state)
                .into_iter()
                .find(|choice| matches!(choice, Choice::Attack { .. }))
                .expect("an Attack only stops where there is a target to take");
            apply(state, &attack);
        }
    }

    #[test]
    fn opening_position_matches_the_printed_board() {
        let state = initial_state();

        // Ten pieces a side: two each of spear, bow, sword and flail, plus the
        // two siege engines.
        for player in 0..NUM_PLAYERS as u8 {
            assert_eq!(state.pieces_of(player).count(), 10);
            assert_eq!(state.siege_engines(player), 2);
            assert_eq!(state.castles_standing(player), 3);
        }

        let back = [
            (2, PieceKind::Spearman),
            (3, PieceKind::Archer),
            (4, PieceKind::Trebuchet),
            (5, PieceKind::Archer),
            (6, PieceKind::Spearman),
        ];
        for (col, kind) in back {
            assert_eq!(
                state.piece_at(Square::new(0, col)),
                Some(Piece { kind, owner: 0 })
            );
            assert_eq!(
                state.piece_at(Square::new(6, col)),
                Some(Piece { kind, owner: 1 })
            );
        }

        // The trebuchet starts standing on its own middle castle, and the
        // castle squares either side of the line hold no piece at all.
        assert_eq!(GameState::castle_slot_at(Square::new(0, 4)), Some((0, 1)));
        assert!(state.piece_at(Square::new(0, 1)).is_none());
        assert!(state.piece_at(Square::new(6, 7)).is_none());

        // The outer column on each wing is empty ground, top to bottom.
        for row in 0..BOARD_ROWS as u8 {
            for col in [0, BOARD_COLS as u8 - 1] {
                assert!(state.piece_at(Square::new(row, col)).is_none());
            }
        }
    }

    #[test]
    fn both_sides_open_with_march_armed_on_the_middle_row_and_flank_on_the_middle_column() {
        let state = initial_state();
        for player in 0..NUM_PLAYERS as u8 {
            assert_eq!(
                state.token(player, TokenKind::March),
                Token {
                    line: 3,
                    side: TokenSide::Attack
                }
            );
            assert_eq!(
                state.token(player, TokenKind::Flank),
                Token {
                    line: 4,
                    side: TokenSide::Move
                }
            );
        }
    }

    #[test]
    fn the_rocks_fill_the_middle_row_between_the_hilltops() {
        let state = initial_state();
        let rocks: Vec<Square> = state
            .pieces()
            .filter(|(_, piece)| piece.kind == PieceKind::Rock)
            .map(|(square, piece)| {
                assert_eq!(piece.owner, NEUTRAL);
                square
            })
            .collect();
        assert_eq!(
            rocks,
            vec![
                Square::new(3, 2),
                Square::new(3, 3),
                Square::new(3, 5),
                Square::new(3, 6),
            ],
        );
        // Hilltop, rock, rock, hilltop, rock, rock, hilltop across the middle.
        for square in rocks {
            assert!(!state.is_hilltop(square));
        }
    }

    #[test]
    fn a_rock_stops_a_move_and_a_killed_one_does_not() {
        let mut state = empty_board();
        place_rock(&mut state, 3, 3);
        place(&mut state, 3, 1, PieceKind::Swordsman, 0);
        move_with(&mut state, TokenKind::March, 3);

        let blocked = destinations(&state);
        assert!(blocked.contains(&Square::new(3, 2)), "up to the rock");
        assert!(!blocked.contains(&Square::new(3, 3)), "never onto it");
        assert!(!blocked.contains(&Square::new(3, 4)), "and never past it");

        state.set_piece(Square::new(3, 3), None);
        let cleared = destinations(&state);
        assert!(cleared.contains(&Square::new(3, 3)));
        assert!(cleared.contains(&Square::new(3, 4)));
    }

    #[test]
    fn an_ordinary_piece_kills_a_rock_and_a_siege_engine_cannot() {
        let mut state = empty_board();
        place_rock(&mut state, 3, 3);
        place(&mut state, 3, 2, PieceKind::BatteringRam, 0);
        place(&mut state, 3, 4, PieceKind::Trebuchet, 0);
        // Siege engines for the other side too, off in the empty wing: without
        // them player 1 has already lost and the attack below ends the game.
        place(&mut state, 0, 0, PieceKind::Trebuchet, 1);
        place(&mut state, 6, 0, PieceKind::BatteringRam, 1);
        state.token_mut(0, TokenKind::March).line = 3;

        // (3,4) is the centre hilltop, so that trebuchet is a live siege
        // weapon — and still has nothing to say to a rock one square away, any
        // more than the ram touching it does.
        assert!(state.is_hilltop(Square::new(3, 4)));
        assert_eq!(
            attack_preview(&state, 0, TokenKind::March).pieces,
            0,
            "the siege train leaves rocks alone"
        );

        place(&mut state, 2, 3, PieceKind::Swordsman, 0);
        state.token_mut(0, TokenKind::Flank).line = 3;
        let (pieces, _) = threatened_by(&state, 0, TokenKind::Flank);
        assert_eq!(pieces, vec![Square::new(3, 3)]);

        arm_attack(&mut state, 0, TokenKind::Flank, 3);
        settle_state(&mut state);
        assert_eq!(
            legal_choices(&state),
            vec![
                Choice::Attack {
                    from: Square::new(2, 3),
                    target: Square::new(3, 3),
                },
                Choice::HoldFire {
                    from: Square::new(2, 3),
                },
            ],
        );

        let events = apply_logged(
            &mut state,
            &Choice::Attack {
                from: Square::new(2, 3),
                target: Square::new(3, 3),
            },
        );
        assert!(events.iter().any(|event| matches!(
            event,
            GameEvent::Struck {
                casualty: Casualty::Piece {
                    piece: Piece {
                        kind: PieceKind::Rock,
                        owner: NEUTRAL
                    }
                },
                ..
            }
        )));
        assert!(state.piece_at(Square::new(3, 3)).is_none());
        assert_eq!(state.outcome, None, "a rock is nobody's to lose");
    }

    #[test]
    fn hilltops_are_the_three_middle_row_squares() {
        let state = initial_state();
        let hilltops: Vec<Square> = GameState::squares()
            .filter(|&square| state.is_hilltop(square))
            .collect();
        assert_eq!(
            hilltops,
            vec![Square::new(3, 1), Square::new(3, 4), Square::new(3, 7)]
        );
    }

    #[test]
    fn each_piece_attacks_its_printed_zone() {
        let cases = [
            (PieceKind::Swordsman, vec![(2, 3), (4, 3), (3, 2), (3, 4)]),
            (PieceKind::Flail, vec![(2, 2), (2, 4), (4, 2), (4, 4)]),
            (PieceKind::Spearman, vec![(1, 3), (5, 3), (3, 1), (3, 5)]),
            (PieceKind::Archer, vec![(1, 1), (1, 5), (5, 1), (5, 5)]),
        ];

        for (kind, zone) in cases {
            let mut state = empty_board();
            place(&mut state, 3, 3, kind, 0);
            // Ring the attacker with enemies at every square within two, so
            // only the zone decides who is in reach.
            for row in 1..=5u8 {
                for col in 1..=5u8 {
                    if (row, col) != (3, 3) {
                        place(&mut state, row, col, PieceKind::Swordsman, 1);
                    }
                }
            }
            state.token_mut(0, TokenKind::March).line = 3;
            let (mut reached, castles) = threatened_by(&state, 0, TokenKind::March);
            let mut expected: Vec<Square> = zone
                .iter()
                .map(|&(row, col)| Square::new(row, col))
                .collect();
            expected.sort();
            reached.sort();
            assert_eq!(reached, expected, "{kind:?} reached the wrong squares");
            assert!(castles.is_empty(), "{kind:?} must not damage a castle");
        }
    }

    #[test]
    fn an_attack_takes_one_target_per_attacker() {
        // Two flails on the same row, and four enemies each of them covers.
        let mut state = empty_board();
        place(&mut state, 3, 1, PieceKind::Flail, 0);
        place(&mut state, 3, 3, PieceKind::Flail, 0);
        for (row, col) in [(2, 0), (2, 2), (4, 2), (4, 4)] {
            place(&mut state, row, col, PieceKind::Swordsman, 1);
        }
        // Siege engines for both sides, off the line: without them a side has
        // already lost and the Attack ends the game mid-way.
        for (col, kind) in [(2, PieceKind::Trebuchet), (3, PieceKind::BatteringRam)] {
            place(&mut state, 0, col, kind, 1);
            place(&mut state, 6, col, kind, 0);
        }
        arm_attack(&mut state, 0, TokenKind::March, 3);
        settle_state(&mut state);

        assert_eq!(state.phase, Phase::Attack);
        attack_at_will(&mut state);

        // Both flails covered three of the four, and each took one.
        assert_eq!(
            state
                .pieces_of(1)
                .filter(|(_, piece)| piece.kind == PieceKind::Swordsman)
                .count(),
            2,
        );
    }

    #[test]
    fn an_attacker_may_hold_its_fire() {
        let mut state = empty_board();
        place(&mut state, 3, 3, PieceKind::Swordsman, 0);
        place(&mut state, 2, 3, PieceKind::Swordsman, 1);
        for (col, kind) in [(2, PieceKind::Trebuchet), (3, PieceKind::BatteringRam)] {
            place(&mut state, 0, col, kind, 1);
            place(&mut state, 6, col, kind, 0);
        }
        arm_attack(&mut state, 0, TokenKind::March, 3);
        settle_state(&mut state);

        let choices = legal_choices(&state);
        assert!(choices.contains(&Choice::Attack {
            from: Square::new(3, 3),
            target: Square::new(2, 3),
        }));
        assert!(choices.contains(&Choice::HoldFire {
            from: Square::new(3, 3)
        }));

        apply(
            &mut state,
            &Choice::HoldFire {
                from: Square::new(3, 3),
            },
        );
        assert!(
            state.piece_at(Square::new(2, 3)).is_some(),
            "holding fire spares the target"
        );
        assert_ne!(state.phase, Phase::Attack, "and ends the Attack");
    }

    #[test]
    fn the_centre_trebuchet_destroys_one_castle_a_turn() {
        let mut state = empty_board();
        place(&mut state, 3, 4, PieceKind::Trebuchet, 0);
        place(&mut state, 6, 3, PieceKind::BatteringRam, 0);
        place(&mut state, 0, 2, PieceKind::Trebuchet, 1);
        place(&mut state, 0, 3, PieceKind::BatteringRam, 1);
        arm_attack(&mut state, 0, TokenKind::March, 3);
        settle_state(&mut state);

        // It bears on all three, and picks one of them.
        assert_eq!(legal_choices(&state).len(), 3 + 1);
        attack_at_will(&mut state);

        assert_eq!(state.castles_standing(1), 2);
        assert_eq!(
            state.outcome, None,
            "one Attack cannot destroy three castles"
        );
    }

    #[test]
    fn a_destroyed_castle_becomes_a_hilltop() {
        let mut state = empty_board();
        state.castles[1] = [false, true, true];
        let rubble = GameState::castle_square(1, 0);
        assert!(state.is_hilltop(rubble));

        // From the rubble it made, a trebuchet bears on the middle castle
        // three squares along the back row.
        place(&mut state, rubble.row, rubble.col, PieceKind::Trebuchet, 0);
        state.token_mut(0, TokenKind::March).line = rubble.row;
        let (_, castles) = threatened_by(&state, 0, TokenKind::March);
        assert_eq!(castles, vec![GameState::castle_square(1, 1)]);

        // And a piece may stand there, which it could not while it stood.
        assert!(!state.blocks_move(rubble, 0));
        assert!(state.blocks_move(GameState::castle_square(1, 1), 0));
    }

    #[test]
    fn a_trebuchet_on_the_centre_hilltop_reaches_every_enemy_castle() {
        let mut state = empty_board();
        place(&mut state, 3, 4, PieceKind::Trebuchet, 0);
        state.token_mut(0, TokenKind::March).line = 3;

        let (pieces, castles) = threatened_by(&state, 0, TokenKind::March);
        assert!(pieces.is_empty(), "a trebuchet does not attack pieces");
        assert_eq!(
            castles,
            vec![Square::new(6, 1), Square::new(6, 4), Square::new(6, 7)],
        );
    }

    #[test]
    fn a_side_hilltop_trebuchet_reaches_two_castles() {
        let mut state = empty_board();
        place(&mut state, 3, 1, PieceKind::Trebuchet, 0);
        state.token_mut(0, TokenKind::March).line = 3;

        let (_, castles) = threatened_by(&state, 0, TokenKind::March);
        // Straight down to (6,1), and diagonally to (6,4).
        assert_eq!(castles, vec![Square::new(6, 1), Square::new(6, 4)]);
    }

    /// The hilltops mark where a trebuchet can attack from, and the rules need
    /// no check to make that so: every square three from a castle already is
    /// one. This holds it true for every square and every set of fallen
    /// castles, so a change to the board that breaks it is caught here.
    #[test]
    fn a_trebuchet_can_only_ever_attack_from_a_hilltop() {
        let mut attacks = 0;
        for destroyed in 0..1u8 << CASTLES_PER_PLAYER {
            for square in GameState::squares() {
                let mut state = empty_board();
                for index in 0..CASTLES_PER_PLAYER {
                    state.castles[1][index] = destroyed >> index & 1 == 0;
                }
                if state.standing_castle_at(square) == Some(1) {
                    continue;
                }
                place(&mut state, square.row, square.col, PieceKind::Trebuchet, 0);
                state.token_mut(0, TokenKind::March).line = square.row;
                let (_, castles) = threatened_by(&state, 0, TokenKind::March);
                if !castles.is_empty() {
                    assert!(state.is_hilltop(square), "{square:?} is not a hilltop");
                    attacks += 1;
                }
            }
        }
        assert!(attacks > 0);
    }

    #[test]
    fn a_battering_ram_destroys_an_adjacent_castle_but_no_pieces() {
        let mut state = empty_board();
        place(&mut state, 5, 4, PieceKind::BatteringRam, 0);
        place(&mut state, 5, 5, PieceKind::Swordsman, 1);
        state.token_mut(0, TokenKind::March).line = 5;

        let (pieces, castles) = threatened_by(&state, 0, TokenKind::March);
        assert_eq!(castles, vec![Square::new(6, 4)]);
        assert!(pieces.is_empty(), "a ram does not fight pieces");
    }

    #[test]
    fn a_ram_diagonal_to_a_castle_reaches_nothing() {
        let mut state = empty_board();
        place(&mut state, 5, 2, PieceKind::BatteringRam, 0);
        state.token_mut(0, TokenKind::March).line = 5;
        let (_, castles) = threatened_by(&state, 0, TokenKind::March);
        // (6,1) is a castle one square away, but on the diagonal.
        assert!(castles.is_empty(), "the ram attacks straight only");
    }

    #[test]
    fn attacks_spare_friendly_pieces_and_already_ruined_castles() {
        let mut state = empty_board();
        place(&mut state, 3, 3, PieceKind::Swordsman, 0);
        place(&mut state, 3, 4, PieceKind::Swordsman, 0);
        place(&mut state, 2, 3, PieceKind::Archer, 1);
        place(&mut state, 3, 1, PieceKind::Trebuchet, 0);
        state.castles[1] = [false, true, false];
        state.token_mut(0, TokenKind::March).line = 3;

        let (pieces, castles) = threatened_by(&state, 0, TokenKind::March);
        assert_eq!(pieces, vec![Square::new(2, 3)]);
        // (6,1) is already rubble; only the standing middle castle is hit.
        assert_eq!(castles, vec![Square::new(6, 4)]);
    }

    #[test]
    fn the_flank_token_attacks_down_its_column() {
        let mut state = empty_board();
        place(&mut state, 2, 4, PieceKind::Flail, 0);
        place(&mut state, 3, 5, PieceKind::Spearman, 1);
        place(&mut state, 3, 3, PieceKind::Spearman, 1);
        state.token_mut(0, TokenKind::Flank).line = 4;

        let (pieces, _) = threatened_by(&state, 0, TokenKind::Flank);
        assert_eq!(pieces, vec![Square::new(3, 3), Square::new(3, 5)]);
        // The same piece is not in the March token's row, so that token hits
        // nothing.
        state.token_mut(0, TokenKind::March).line = 0;
        assert!(attack_preview(&state, 0, TokenKind::March).is_empty());
    }

    #[test]
    fn the_first_turn_is_played_like_every_other() {
        let mut state = initial_state();
        assert_eq!(state.sliding_token(), TokenKind::Flank);

        apply(&mut state, &Choice::Slide { line: 2 });
        assert_eq!(state.phase, Phase::Order, "both tokens activate");
        assert_eq!(
            state.pending_tokens(),
            &[TokenKind::Flank, TokenKind::March]
        );

        // Flank the swordsman on (1,2) onto the centre hilltop, and the March
        // token's Attack on the middle row then has the rocks either side of it.
        apply(
            &mut state,
            &Choice::Order {
                first: TokenKind::Flank,
            },
        );
        apply(
            &mut state,
            &Choice::Move {
                from: Square::new(1, 2),
                to: Square::new(3, 4),
            },
        );
        assert_eq!(state.phase, Phase::Attack);
        assert_eq!(
            legal_choices(&state),
            vec![
                Choice::Attack {
                    from: Square::new(3, 4),
                    target: Square::new(3, 3),
                },
                Choice::Attack {
                    from: Square::new(3, 4),
                    target: Square::new(3, 5),
                },
                Choice::HoldFire {
                    from: Square::new(3, 4),
                },
            ],
        );

        apply(
            &mut state,
            &Choice::Attack {
                from: Square::new(3, 4),
                target: Square::new(3, 3),
            },
        );
        assert!(state.piece_at(Square::new(3, 3)).is_none());
        assert_eq!(state.current_player, 1);
        assert_eq!(state.token(0, TokenKind::March).side, TokenSide::Move);
        assert_eq!(state.token(0, TokenKind::Flank).side, TokenSide::Attack);
    }

    #[test]
    fn the_sliding_token_alternates_from_turn_to_turn() {
        let mut state = initial_state();
        for player in 0..2u8 {
            assert_eq!(state.current_player, player);
            assert_eq!(state.sliding_token(), TokenKind::Flank);
            apply(&mut state, &Choice::Slide { line: 0 });
            // The March Attack has no piece on its row, so the Flank activation
            // is all that is left to decide.
            apply(
                &mut state,
                &Choice::Order {
                    first: TokenKind::March,
                },
            );
            assert_eq!(state.phase, Phase::Activate);
            assert_eq!(state.pending_tokens(), &[TokenKind::Flank]);
            apply(&mut state, &Choice::Pass);
        }

        assert_eq!(state.sliding_token(), TokenKind::March);
        let slides = legal_choices(&state);
        assert_eq!(
            slides.len(),
            BOARD_ROWS - 1,
            "every row but the one it is on"
        );
        assert!(!slides.contains(&Choice::Slide { line: 3 }));
    }

    #[test]
    fn exactly_one_token_shows_its_move_side_when_a_turn_begins() {
        let mut state = initial_state();
        let mut rng_index = 0usize;
        let mut choices = Vec::new();

        for _ in 0..200 {
            if state.outcome.is_some() {
                break;
            }
            if state.phase == Phase::Slide {
                let player = state.current_player;
                assert_ne!(
                    state.token(player, TokenKind::March).side,
                    state.token(player, TokenKind::Flank).side,
                );
            }
            legal_choices_into(&state, &mut choices);
            let choice = choices[rng_index % choices.len()];
            rng_index += 7;
            apply(&mut state, &choice);
        }
    }

    #[test]
    fn a_slide_goes_to_any_other_line() {
        // Columns 0, 1, 7 and 8 hold none of Ivory's pieces and are open all
        // the same: an empty line armed now can be moved onto next turn.
        let choices = legal_choices(&initial_state());
        let expected: Vec<Choice> = [0, 1, 2, 3, 5, 6, 7, 8]
            .into_iter()
            .map(|line| Choice::Slide { line })
            .collect();
        assert_eq!(choices, expected);
    }

    #[test]
    fn moving_never_captures_and_never_jumps() {
        let mut state = empty_board();
        place(&mut state, 3, 3, PieceKind::Swordsman, 0);
        place(&mut state, 3, 5, PieceKind::Swordsman, 1);
        place(&mut state, 1, 3, PieceKind::Flail, 0);
        move_with(&mut state, TokenKind::March, 3);

        let destinations: Vec<Square> = legal_choices(&state)
            .into_iter()
            .filter_map(|choice| match choice {
                Choice::Move { from, to } if from == Square::new(3, 3) => Some(to),
                _ => None,
            })
            .collect();

        assert!(
            destinations.contains(&Square::new(3, 4)),
            "up to the blocker"
        );
        assert!(
            !destinations.contains(&Square::new(3, 5)),
            "a move never captures"
        );
        assert!(
            !destinations.contains(&Square::new(3, 6)),
            "and never jumps the blocker"
        );
        assert!(destinations.contains(&Square::new(2, 3)));
        assert!(
            !destinations.contains(&Square::new(1, 3)),
            "friendly pieces block too"
        );
    }

    #[test]
    fn an_enemy_castle_stops_a_move_and_a_friendly_one_does_not() {
        let mut state = empty_board();
        // On the enemy back row, between their outer and middle castles.
        place(&mut state, 6, 2, PieceKind::Swordsman, 0);
        move_with(&mut state, TokenKind::March, 6);

        let destinations = destinations(&state);
        assert!(
            !destinations.contains(&Square::new(6, 1)),
            "cannot enter it"
        );
        assert!(destinations.contains(&Square::new(6, 3)));
        assert!(
            !destinations.contains(&Square::new(6, 4)),
            "the middle castle is enemy ground too"
        );
        assert!(
            !destinations.contains(&Square::new(6, 5)),
            "and a move does not pass through it"
        );

        // Its own castles are open ground: the trebuchet opens standing on one.
        let mut own = empty_board();
        place(&mut own, 0, 2, PieceKind::Swordsman, 0);
        move_with(&mut own, TokenKind::March, 0);
        assert!(legal_choices(&own).contains(&Choice::Move {
            from: Square::new(0, 2),
            to: Square::new(0, 1),
        }));
    }

    #[test]
    fn march_moves_straight_and_flank_moves_diagonally() {
        for (kind, straight) in [(TokenKind::March, true), (TokenKind::Flank, false)] {
            let mut state = empty_board();
            place(&mut state, 3, 3, PieceKind::Swordsman, 0);
            move_with(&mut state, kind, 3);
            let destinations = destinations(&state);

            // From (3,3) on an empty 9x7 board a March reaches 8 squares along
            // the row and 6 up the column; a Flank runs three squares to each
            // of the four edges. Neither line meets a castle on the way.
            assert_eq!(
                destinations.len(),
                if straight { 14 } else { 12 },
                "{kind:?}"
            );
            let has_straight = destinations.contains(&Square::new(3, 8));
            let has_diagonal = destinations.contains(&Square::new(0, 0));
            assert_eq!(has_straight, straight, "{kind:?} straight");
            assert_eq!(has_diagonal, !straight, "{kind:?} diagonal");
        }
    }

    #[test]
    fn a_piece_may_leave_the_line_its_token_named() {
        // "Move 1 piece in the row vertically or horizontally" picks the piece
        // from the row; the move itself may carry it out of the row entirely.
        let mut state = empty_board();
        place(&mut state, 3, 3, PieceKind::Swordsman, 0);
        move_with(&mut state, TokenKind::March, 3);

        assert!(legal_choices(&state).contains(&Choice::Move {
            from: Square::new(3, 3),
            to: Square::new(0, 3),
        }));
    }

    #[test]
    fn destroying_the_last_castle_ends_the_game() {
        let mut state = empty_board();
        place(&mut state, 5, 4, PieceKind::BatteringRam, 0);
        place(&mut state, 0, 3, PieceKind::Trebuchet, 0);
        place(&mut state, 6, 2, PieceKind::Trebuchet, 1);
        place(&mut state, 6, 3, PieceKind::BatteringRam, 1);
        state.castles[1] = [false, true, false];
        arm_attack(&mut state, 0, TokenKind::March, 5);

        // The queued Attack is the only thing left this turn, so taking its one
        // target ends the turn — and the game.
        settle_state(&mut state);
        attack_at_will(&mut state);
        assert_eq!(state.outcome, Some(Outcome::Winner { player: 0 }));
        assert_eq!(state.phase, Phase::GameOver);
        assert!(legal_choices(&state).is_empty());
    }

    #[test]
    fn losing_both_siege_engines_ends_the_game() {
        // One attacker per engine: a single piece could only take one of them.
        let mut state = empty_board();
        place(&mut state, 3, 1, PieceKind::Swordsman, 0);
        place(&mut state, 3, 3, PieceKind::Swordsman, 0);
        place(&mut state, 0, 2, PieceKind::Trebuchet, 0);
        place(&mut state, 0, 3, PieceKind::BatteringRam, 0);
        place(&mut state, 3, 2, PieceKind::Trebuchet, 1);
        place(&mut state, 3, 4, PieceKind::BatteringRam, 1);
        arm_attack(&mut state, 0, TokenKind::March, 3);

        settle_state(&mut state);
        attack_at_will(&mut state);
        assert_eq!(
            state.outcome,
            Some(Outcome::Winner { player: 0 }),
            "a side with no siege engine can never destroy a castle, so it has lost"
        );
    }

    #[test]
    fn one_surviving_siege_engine_is_not_a_loss() {
        let mut state = empty_board();
        place(&mut state, 3, 3, PieceKind::Swordsman, 0);
        place(&mut state, 3, 2, PieceKind::Trebuchet, 1);
        place(&mut state, 6, 2, PieceKind::BatteringRam, 1);
        place(&mut state, 0, 2, PieceKind::Trebuchet, 0);
        place(&mut state, 0, 3, PieceKind::BatteringRam, 0);
        arm_attack(&mut state, 0, TokenKind::March, 3);

        settle_state(&mut state);
        attack_at_will(&mut state);
        assert_eq!(state.outcome, None);
        assert_eq!(state.siege_engines(1), 1);
    }

    #[test]
    fn a_move_side_with_nowhere_to_go_passes() {
        let mut state = empty_board();
        // Boxed into a corner by pieces on both straight lines.
        place(&mut state, 0, 0, PieceKind::Swordsman, 0);
        place(&mut state, 0, 1, PieceKind::Swordsman, 1);
        place(&mut state, 1, 0, PieceKind::Swordsman, 1);
        move_with(&mut state, TokenKind::March, 0);

        assert_eq!(legal_choices(&state), vec![Choice::Pass]);
    }

    #[test]
    fn a_move_side_may_always_decline_to_move() {
        let mut state = empty_board();
        place(&mut state, 3, 3, PieceKind::Swordsman, 0);
        move_with(&mut state, TokenKind::March, 3);

        let choices = legal_choices(&state);
        assert!(choices.len() > 1, "the piece has somewhere to go");
        assert!(
            choices.contains(&Choice::Pass),
            "and may still flip the token and take no action"
        );
    }

    #[test]
    fn every_position_reachable_by_random_play_offers_a_legal_choice() {
        let mut seed = 0x5EEDu64;
        for _ in 0..200 {
            let mut state = initial_state();
            let mut choices = Vec::new();
            let mut steps = 0;
            while state.outcome.is_none() {
                legal_choices_into(&state, &mut choices);
                assert!(
                    !choices.is_empty(),
                    "no legal choice in {:?} for player {}",
                    state.phase,
                    state.current_player
                );
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let choice = choices[(seed >> 33) as usize % choices.len()];
                apply(&mut state, &choice);
                steps += 1;
                assert!(steps < 20_000, "a random game failed to terminate");
            }
            // Whatever ended it, the position agrees with the verdict.
            match state.outcome.unwrap() {
                Outcome::Winner { player } => {
                    let loser = GameState::opponent(player);
                    assert!(state.castles_standing(loser) == 0 || state.siege_engines(loser) == 0);
                }
                Outcome::Draw => assert!(state.turn >= MAX_TURNS),
            }
        }
    }
}
