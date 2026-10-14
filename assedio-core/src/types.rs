//! Board geometry, pieces, command tokens, and the game state they live in.
//!
//! `GameState` is `Copy` on purpose: the search clones one per determinization
//! and per rollout step, and a memcpy of under 200 bytes is cheaper than
//! anything that chases a pointer. That is why the board is a fixed array with
//! its occupancy kept as bitboards beside it, and the activation queue is an
//! array plus a length rather than a `Vec`.

use serde::{Deserialize, Serialize};

/// The board is wider than it is deep: nine columns across, seven rows up. The
/// outermost columns carry no terrain and start empty, which is what gives the
/// middle row room for the rocks.
pub const BOARD_ROWS: usize = 7;
pub const BOARD_COLS: usize = 9;
pub const BOARD_SQUARES: usize = BOARD_ROWS * BOARD_COLS;
pub const NUM_PLAYERS: usize = 2;
pub const CASTLES_PER_PLAYER: usize = 3;

/// The owner of a rock, which belongs to neither side.
pub const NEUTRAL: u8 = 2;

/// Back row for each player: where their castles stand.
pub const BACK_ROW: [u8; NUM_PLAYERS] = [0, 6];

/// Columns carrying a castle on a player's back row.
pub const CASTLE_COLS: [u8; CASTLES_PER_PLAYER] = [1, 4, 7];

/// The middle row, and the columns on it that are hilltops. Castles and
/// hilltops share columns, which is what puts every castle exactly three
/// squares from a hilltop — straight for the near ones, diagonal for the rest.
pub const HILLTOP_ROW: u8 = 3;
pub const HILLTOP_COLS: [u8; 3] = CASTLE_COLS;

/// The rocks fill the middle row between the hilltops.
pub const ROCK_COLS: [u8; 4] = [2, 3, 5, 6];

/// Turns each side may take before the game is called a draw. Nothing in the
/// rules forces progress, and a search needs every line to terminate.
pub const MAX_TURNS: u32 = 300;

/// One bit per square, in [`Square::index`] order. The board has 63 squares,
/// so a whole-board set is one `u64` and set arithmetic is one instruction.
pub type Bitboard = u64;

/// The squares in `bits`, in [`Square::index`] order.
pub fn squares_of(mut bits: Bitboard) -> impl Iterator<Item = Square> {
    std::iter::from_fn(move || {
        (bits != 0).then(|| {
            let index = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Square::from_index(index)
        })
    })
}

const fn square_bit(row: usize, col: usize) -> Bitboard {
    1 << (row * BOARD_COLS + col)
}

const fn line_masks() -> [[Bitboard; BOARD_COLS]; 2] {
    let mut masks = [[0; BOARD_COLS]; 2];
    let mut row = 0;
    while row < BOARD_ROWS {
        let mut col = 0;
        while col < BOARD_COLS {
            masks[TokenKind::March.index()][row] |= square_bit(row, col);
            masks[TokenKind::Flank.index()][col] |= square_bit(row, col);
            col += 1;
        }
        row += 1;
    }
    masks
}

const LINE_MASKS: [[Bitboard; BOARD_COLS]; 2] = line_masks();

const fn zone_masks(kind: PieceKind) -> [Bitboard; BOARD_SQUARES] {
    let zone = kind.attack_zone();
    let mut masks = [0; BOARD_SQUARES];
    let mut index = 0;
    while index < BOARD_SQUARES {
        let mut i = 0;
        while i < zone.len() {
            let row = (index / BOARD_COLS) as i8 + zone[i].0;
            let col = (index % BOARD_COLS) as i8 + zone[i].1;
            if row >= 0 && row < BOARD_ROWS as i8 && col >= 0 && col < BOARD_COLS as i8 {
                masks[index] |= square_bit(row as usize, col as usize);
            }
            i += 1;
        }
        index += 1;
    }
    masks
}

/// Indexed by `PieceKind as usize`, then by square.
const ZONE_MASKS: [[Bitboard; BOARD_SQUARES]; 7] = [
    zone_masks(PieceKind::Swordsman),
    zone_masks(PieceKind::Flail),
    zone_masks(PieceKind::Spearman),
    zone_masks(PieceKind::Archer),
    zone_masks(PieceKind::Trebuchet),
    zone_masks(PieceKind::BatteringRam),
    zone_masks(PieceKind::Rock),
];

const NATURAL_HILLTOPS: Bitboard = square_bit(HILLTOP_ROW as usize, HILLTOP_COLS[0] as usize)
    | square_bit(HILLTOP_ROW as usize, HILLTOP_COLS[1] as usize)
    | square_bit(HILLTOP_ROW as usize, HILLTOP_COLS[2] as usize);

const NO_CASTLE: u8 = u8::MAX;

/// For each square, `owner * CASTLES_PER_PLAYER + index` of the castle slot it
/// holds, or [`NO_CASTLE`].
const fn castle_slots() -> [u8; BOARD_SQUARES] {
    let mut slots = [NO_CASTLE; BOARD_SQUARES];
    let mut owner = 0;
    while owner < NUM_PLAYERS {
        let mut index = 0;
        while index < CASTLES_PER_PLAYER {
            let square = BACK_ROW[owner] as usize * BOARD_COLS + CASTLE_COLS[index] as usize;
            slots[square] = (owner * CASTLES_PER_PLAYER + index) as u8;
            index += 1;
        }
        owner += 1;
    }
    slots
}

const CASTLE_SLOTS: [u8; BOARD_SQUARES] = castle_slots();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PieceKind {
    Swordsman,
    Flail,
    Spearman,
    Archer,
    Trebuchet,
    BatteringRam,
    /// Neutral: it never moves or attacks, it blocks a move like any piece, and
    /// either side's ordinary pieces may kill it.
    Rock,
}

const SWORDSMAN_ZONE: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const FLAIL_ZONE: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const SPEARMAN_ZONE: [(i8, i8); 4] = [(-2, 0), (2, 0), (0, -2), (0, 2)];
const ARCHER_ZONE: [(i8, i8); 4] = [(-2, -2), (-2, 2), (2, -2), (2, 2)];
const TREBUCHET_ZONE: [(i8, i8); 8] = [
    (-3, 0),
    (3, 0),
    (0, -3),
    (0, 3),
    (-3, -3),
    (-3, 3),
    (3, -3),
    (3, 3),
];
const RAM_ZONE: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

impl PieceKind {
    /// The piece's Attack Zone, relative to where it stands. Nothing blocks an
    /// attack: a spearman reaches over the square between it and its target.
    pub const fn attack_zone(self) -> &'static [(i8, i8)] {
        match self {
            PieceKind::Swordsman => &SWORDSMAN_ZONE,
            PieceKind::Flail => &FLAIL_ZONE,
            PieceKind::Spearman => &SPEARMAN_ZONE,
            PieceKind::Archer => &ARCHER_ZONE,
            PieceKind::Trebuchet => &TREBUCHET_ZONE,
            PieceKind::BatteringRam => &RAM_ZONE,
            PieceKind::Rock => &[],
        }
    }

    /// The on-board squares of the piece's Attack Zone from `from`.
    #[inline]
    pub fn zone_mask(self, from: Square) -> Bitboard {
        ZONE_MASKS[self as usize][from.index()]
    }

    /// Siege engines destroy castles and nothing else; every other piece kills
    /// pieces and nothing else. No piece does both.
    pub fn is_siege(self) -> bool {
        matches!(self, PieceKind::Trebuchet | PieceKind::BatteringRam)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Piece {
    pub kind: PieceKind,
    /// A player, or [`NEUTRAL`] for a rock.
    pub owner: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Square {
    pub row: u8,
    pub col: u8,
}

impl Square {
    pub const fn new(row: u8, col: u8) -> Self {
        Self { row, col }
    }

    /// The square `(drow, dcol)` away, or `None` if that falls off the board.
    pub fn offset(self, drow: i8, dcol: i8) -> Option<Square> {
        let row = self.row as i8 + drow;
        let col = self.col as i8 + dcol;
        let on_board = (0..BOARD_ROWS as i8).contains(&row) && (0..BOARD_COLS as i8).contains(&col);
        on_board.then(|| Square::new(row as u8, col as u8))
    }

    #[inline]
    pub fn index(self) -> usize {
        self.row as usize * BOARD_COLS + self.col as usize
    }

    #[inline]
    pub fn from_index(index: usize) -> Square {
        Square::new((index / BOARD_COLS) as u8, (index % BOARD_COLS) as u8)
    }

    #[inline]
    pub fn bit(self) -> Bitboard {
        1 << self.index()
    }

    pub fn chebyshev(self, other: Square) -> u8 {
        let drow = self.row.abs_diff(other.row);
        let dcol = self.col.abs_diff(other.col);
        drow.max(dcol)
    }
}

/// Which of a player's two command tokens, named for its Move side.
///
/// The `March` token rides the left or right edge and names a row; its Move
/// side moves a piece from that row vertically or horizontally. The `Flank`
/// token rides the top or bottom edge and names a column; its Move side moves a
/// piece from that column diagonally. Both share the same Attack side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenKind {
    March,
    Flank,
}

pub const TOKEN_KINDS: [TokenKind; 2] = [TokenKind::March, TokenKind::Flank];

impl TokenKind {
    pub const fn index(self) -> usize {
        match self {
            TokenKind::March => 0,
            TokenKind::Flank => 1,
        }
    }

    pub fn other(self) -> TokenKind {
        match self {
            TokenKind::March => TokenKind::Flank,
            TokenKind::Flank => TokenKind::March,
        }
    }
}

/// Which side of a command token is face up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenSide {
    Move,
    Attack,
}

impl TokenSide {
    pub fn flipped(self) -> TokenSide {
        match self {
            TokenSide::Move => TokenSide::Attack,
            TokenSide::Attack => TokenSide::Move,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    /// Row index for the March token, column index for the Flank token.
    pub line: u8,
    pub side: TokenSide,
}

/// Where a turn is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// Slide the token showing its Move side to a new line.
    Slide,
    /// Choose which of the two tokens activates first.
    Order,
    /// Spend the Move side at the head of the queue.
    Activate,
    /// Aim one attacker of an Attack. Each piece on the line acts in turn, so
    /// an Attack side costs as many decisions as it has pieces with a target.
    Attack,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Choice {
    /// Slide the token showing its Move side to `line`. Exactly one token shows
    /// it when a turn begins, so the line is the whole decision.
    Slide { line: u8 },
    /// Activate `first` before the other token.
    Order { first: TokenKind },
    /// Spend a Move side moving a piece.
    Move { from: Square, to: Square },
    /// Spend a Move side without moving.
    Pass,
    /// The attacker on `from` kills or destroys what stands on `target`. An
    /// attacker takes one target, not everything in its Attack Zone.
    Attack { from: Square, target: Square },
    /// Leave the attacker on `from` idle and move on down the line.
    HoldFire { from: Square },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Outcome {
    Winner { player: u8 },
    Draw,
}

/// Every piece on the board, the neutral rocks included, with each owner's
/// squares kept beside them as a bitboard. "Any piece of mine on this line" and
/// "anything to take in this zone" are then a mask and a compare rather than a
/// walk over squares.
///
/// Serialized as rows of cells, so the wire format never sees the bitboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Board {
    cells: [Option<Piece>; BOARD_SQUARES],
    /// Indexed by owner: each player, then [`NEUTRAL`].
    occupied_by: [Bitboard; NUM_PLAYERS + 1],
}

impl Board {
    pub const EMPTY: Board = Board {
        cells: [None; BOARD_SQUARES],
        occupied_by: [0; NUM_PLAYERS + 1],
    };

    #[inline]
    pub fn get(&self, square: Square) -> Option<Piece> {
        self.cells[square.index()]
    }

    #[inline]
    pub fn set(&mut self, square: Square, piece: Option<Piece>) {
        if let Some(old) = self.cells[square.index()] {
            self.occupied_by[old.owner as usize] &= !square.bit();
        }
        if let Some(new) = piece {
            self.occupied_by[new.owner as usize] |= square.bit();
        }
        self.cells[square.index()] = piece;
    }

    /// The squares holding `owner`'s pieces: a player's, or [`NEUTRAL`]'s rocks.
    #[inline]
    pub fn occupied_by(&self, owner: u8) -> Bitboard {
        self.occupied_by[owner as usize]
    }

    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.occupied_by.iter().fold(0, |all, &owned| all | owned)
    }
}

impl Serialize for Board {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut rows = [[None; BOARD_COLS]; BOARD_ROWS];
        for (index, cell) in self.cells.iter().enumerate() {
            rows[index / BOARD_COLS][index % BOARD_COLS] = *cell;
        }
        rows.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Board {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = <[[Option<Piece>; BOARD_COLS]; BOARD_ROWS]>::deserialize(deserializer)?;
        let mut board = Board::EMPTY;
        for (index, &cell) in rows.as_flattened().iter().enumerate() {
            board.set(Square::from_index(index), cell);
        }
        Ok(board)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    /// Every piece on the board, the neutral rocks included.
    pub board: Board,
    /// `castles[player][i]` is whether the castle on `CASTLE_COLS[i]` still
    /// stands. Castles are terrain, not pieces: they never move, never attack,
    /// and a piece may stand on its own.
    pub castles: [[bool; CASTLES_PER_PLAYER]; NUM_PLAYERS],
    /// Indexed by player, then by `TokenKind::index`.
    pub tokens: [[Token; 2]; NUM_PLAYERS],
    pub current_player: u8,
    pub phase: Phase,
    /// Tokens still to activate this turn, head first. Only the first
    /// `pending_len` entries are meaningful.
    pub pending: [TokenKind; 2],
    pub pending_len: u8,
    /// Under `Phase::Attack`, how far along the token's line the Attack has
    /// got: the index of the piece now choosing its target. Meaningless in
    /// every other phase.
    pub attack_index: u8,
    /// Full turns taken by both sides so far.
    pub turn: u32,
    pub outcome: Option<Outcome>,
}

impl GameState {
    #[inline]
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.board.get(square)
    }

    #[inline]
    pub fn set_piece(&mut self, square: Square, piece: Option<Piece>) {
        self.board.set(square, piece);
    }

    pub fn token(&self, player: u8, kind: TokenKind) -> Token {
        self.tokens[player as usize][kind.index()]
    }

    pub fn token_mut(&mut self, player: u8, kind: TokenKind) -> &mut Token {
        &mut self.tokens[player as usize][kind.index()]
    }

    /// The token the current player slides this turn: the one showing its Move
    /// side. Every turn flips both tokens and the game starts with one on each
    /// side, so exactly one shows it whenever a turn begins.
    pub fn sliding_token(&self) -> TokenKind {
        TOKEN_KINDS
            .into_iter()
            .find(|&kind| self.token(self.current_player, kind).side == TokenSide::Move)
            .expect("a turn begins with one token on its Move side")
    }

    pub fn opponent(player: u8) -> u8 {
        1 - player
    }

    pub fn squares() -> impl Iterator<Item = Square> {
        (0..BOARD_ROWS as u8)
            .flat_map(|row| (0..BOARD_COLS as u8).map(move |col| Square::new(row, col)))
    }

    /// How many lines of this kind the board has: seven rows for the March
    /// token, nine columns for the Flank token.
    pub fn line_count(kind: TokenKind) -> u8 {
        match kind {
            TokenKind::March => BOARD_ROWS as u8,
            TokenKind::Flank => BOARD_COLS as u8,
        }
    }

    /// How many squares sit on one such line — the other dimension.
    pub fn line_len(kind: TokenKind) -> u8 {
        Self::line_count(kind.other())
    }

    /// The squares of `line`, which is a row for the March token and a column
    /// for the Flank token.
    pub fn line_squares(kind: TokenKind, line: u8) -> impl Iterator<Item = Square> {
        (0..Self::line_len(kind)).map(move |index| Self::line_square(kind, line, index))
    }

    /// The squares of `line`, as a bitboard.
    #[inline]
    pub fn line_mask(kind: TokenKind, line: u8) -> Bitboard {
        LINE_MASKS[kind.index()][line as usize]
    }

    /// The `index`th square of `line`, counting along the line.
    pub fn line_square(kind: TokenKind, line: u8, index: u8) -> Square {
        match kind {
            TokenKind::March => Square::new(line, index),
            TokenKind::Flank => Square::new(index, line),
        }
    }

    /// One of the three hilltops printed on the board.
    pub fn is_natural_hilltop(square: Square) -> bool {
        NATURAL_HILLTOPS & square.bit() != 0
    }

    /// A hilltop as the board stands: one of the printed three, or a castle
    /// that has been destroyed and become a hilltop in its place.
    pub fn is_hilltop(&self, square: Square) -> bool {
        Self::is_natural_hilltop(square) || self.destroyed_castle_at(square)
    }

    fn destroyed_castle_at(&self, square: Square) -> bool {
        matches!(
            Self::castle_slot_at(square),
            Some((owner, index)) if !self.castles[owner as usize][index]
        )
    }

    /// Who owns the castle still standing on `square`, if one does.
    pub fn standing_castle_at(&self, square: Square) -> Option<u8> {
        match Self::castle_slot_at(square) {
            Some((owner, index)) if self.castles[owner as usize][index] => Some(owner),
            _ => None,
        }
    }

    /// Whether terrain on `square` stops `player`'s move: a piece may neither
    /// enter a standing enemy castle nor pass through one. Its own castles are
    /// open ground.
    pub fn blocks_move(&self, square: Square, player: u8) -> bool {
        self.standing_castle_at(square)
            .is_some_and(|owner| owner != player)
    }

    /// The castle slot `square` belongs to, if any, as `(owner, index)`.
    /// Reports the slot whether or not that castle still stands.
    #[inline]
    pub fn castle_slot_at(square: Square) -> Option<(u8, usize)> {
        let slot = CASTLE_SLOTS[square.index()];
        (slot != NO_CASTLE).then(|| {
            let owner = slot / CASTLES_PER_PLAYER as u8;
            (owner, slot as usize % CASTLES_PER_PLAYER)
        })
    }

    /// `owner`'s standing castles, as a bitboard.
    pub fn standing_castles(&self, owner: u8) -> Bitboard {
        (0..CASTLES_PER_PLAYER)
            .filter(|&index| self.castles[owner as usize][index])
            .fold(0, |castles, index| {
                castles | Self::castle_square(owner, index).bit()
            })
    }

    pub fn castle_square(owner: u8, index: usize) -> Square {
        Square::new(BACK_ROW[owner as usize], CASTLE_COLS[index])
    }

    pub fn castles_standing(&self, player: u8) -> usize {
        self.castles[player as usize].iter().filter(|&&c| c).count()
    }

    pub fn pieces(&self) -> impl Iterator<Item = (Square, Piece)> + '_ {
        self.pieces_on(self.board.occupied())
    }

    pub fn pieces_of(&self, player: u8) -> impl Iterator<Item = (Square, Piece)> + '_ {
        self.pieces_on(self.board.occupied_by(player))
    }

    fn pieces_on(&self, squares: Bitboard) -> impl Iterator<Item = (Square, Piece)> + '_ {
        squares_of(squares).map(|square| {
            let piece = self
                .piece_at(square)
                .expect("a square in an occupancy bitboard holds a piece");
            (square, piece)
        })
    }

    pub fn siege_engines(&self, player: u8) -> usize {
        self.pieces_of(player)
            .filter(|(_, piece)| piece.kind.is_siege())
            .count()
    }

    pub fn pending_tokens(&self) -> &[TokenKind] {
        &self.pending[..self.pending_len as usize]
    }
}
