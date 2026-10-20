// Mirrors the serde output of `assedio-core`. The shapes here are pinned on the
// Rust side by `assedio-core/tests/wire_format.rs`; change one and change the
// other.

/** Nine columns across, nine rows up: the outer column on each wing is empty,
    and each side's back row lies behind its castles. */
export const BOARD_ROWS = 9
export const BOARD_COLS = 9
export const CASTLE_COLS = [1, 4, 7]
export const CASTLE_ROW = [1, 7]
export const HILLTOP_ROW = 4
export const HILLTOP_COLS = CASTLE_COLS
/** The owner of a rock, which belongs to neither side. */
export const NEUTRAL = 2
export const COLUMN_LETTERS = 'abcdefghi'

export type PieceKind =
  | 'swordsman'
  | 'flail'
  | 'spearman'
  | 'archer'
  | 'trebuchet'
  | 'batteringRam'
  | 'rock'

export interface Piece {
  kind: PieceKind
  /** A player, or `NEUTRAL` for a rock. */
  owner: number
}

export interface Square {
  row: number
  col: number
}

export type TokenKind = 'march' | 'flank'
export type TokenSide = 'move' | 'attack'

export interface Token {
  line: number
  side: TokenSide
}

export type Phase = 'slide' | 'order' | 'activate' | 'attack' | 'gameOver'

export type Choice =
  | { type: 'slide'; line: number }
  | { type: 'order'; first: TokenKind }
  | { type: 'move'; from: Square; to: Square }
  | { type: 'pass' }
  | { type: 'attack'; from: Square; target: Square }
  | { type: 'holdFire'; from: Square }

/** What one attacker took: a piece, rocks included, or a castle. */
export type Casualty = { type: 'piece'; piece: Piece } | { type: 'castle' }

export type Outcome = { type: 'winner'; player: number } | { type: 'draw' }

export interface GameState {
  /** Every piece on the board, the neutral rocks included. */
  board: (Piece | null)[][]
  castles: boolean[][]
  /** Indexed by player, then by `tokenIndex` — 0 is the March token. */
  tokens: Token[][]
  currentPlayer: number
  phase: Phase
  pending: TokenKind[]
  pendingLen: number
  /** How far along its line an Attack has got. Only meaningful in `attack`. */
  attackIndex: number
  turn: number
  outcome: Outcome | null
}

export type GameEvent =
  | { type: 'slid'; player: number; token: TokenKind; from: number; to: number }
  | {
      type: 'moved'
      player: number
      token: TokenKind
      kind: PieceKind
      from: Square
      to: Square
    }
  | { type: 'passed'; player: number; token: TokenKind }
  | { type: 'attacked'; player: number; token: TokenKind; line: number }
  | {
      type: 'struck'
      player: number
      token: TokenKind
      kind: PieceKind
      from: Square
      target: Square
      casualty: Casualty
    }
  | {
      type: 'heldFire'
      player: number
      token: TokenKind
      kind: PieceKind
      from: Square
    }
  | { type: 'turnEnded'; player: number }
  | { type: 'gameOver'; outcome: Outcome }

/** What one Attack bears on, and in what targets. Threatened pieces include
    rocks: either side may kill one. */
export interface Threat {
  threatenedPieces: Square[]
  threatenedCastles: Square[]
}

/** An Attack's reach: each attacker takes one of these, not all of them. */
export interface AttackPreview extends Threat {
  attackers: Square[]
  /** The Attack Zone of the line's pieces, occupied or not. */
  zone: Square[]
}

/** What one move puts within reach of the Attack still to come this turn. */
export interface MoveOutcome extends Threat {
  from: Square
  to: Square
}

/** What sliding the token to a line sets up, now and next turn. */
export interface SlideOutcome extends Threat {
  token: TokenKind
  line: number
  movers: Square[]
  zone: Square[]
}

/** One piece's Attack Zone and what in it the piece could take today. A siege
    engine's zone is only the enemy castles in range. */
export interface PieceReach {
  zone: Square[]
  targets: Square[]
}

/** One attacker on an armed line and everything it could take. */
export interface Shot {
  from: Square
  kind: PieceKind
  targets: Square[]
}

export const TOKEN_KINDS: TokenKind[] = ['march', 'flank']

export const TOKEN_NAMES: Record<TokenKind, string> = {
  march: 'March',
  flank: 'Flank',
}

function tokenIndex(kind: TokenKind): number {
  return kind === 'march' ? 0 : 1
}

export function tokenOf(state: GameState, player: number, kind: TokenKind): Token {
  return state.tokens[player][tokenIndex(kind)]
}

export function opponent(player: number): number {
  return 1 - player
}

export function sameSquare(a: Square, b: Square): boolean {
  return a.row === b.row && a.col === b.col
}

export function squareKey(square: Square): string {
  return `${square.row},${square.col}`
}

export function pieceAt(state: GameState, square: Square): Piece | null {
  return state.board[square.row][square.col]
}

/** Tokens still waiting to activate this turn, in the order they will act. */
export function pendingTokens(state: GameState): TokenKind[] {
  return state.pending.slice(0, state.pendingLen)
}

/** The token whose Attack `player` makes next: mid-turn, the one still waiting
    to act Attack side up; otherwise the one showing its Attack side. Mid-turn
    both can show it — a Move side flips to Attack once it has acted — so the
    side alone does not say which is still to come. */
export function attackingToken(state: GameState, player: number): TokenKind | null {
  if (player === state.currentPlayer && state.phase !== 'slide') {
    return pendingTokens(state).find((kind) => tokenOf(state, player, kind).side === 'attack') ?? null
  }
  return TOKEN_KINDS.find((kind) => tokenOf(state, player, kind).side === 'attack') ?? null
}

/** One of the printed three, or a destroyed castle, which becomes one. */
export function isHilltop(state: GameState, square: Square): boolean {
  if (square.row === HILLTOP_ROW && HILLTOP_COLS.includes(square.col)) return true
  const slot = castleSlotAt(square)
  return slot !== null && !state.castles[slot.owner][slot.index]
}

/** The castle slot on `square`, whether or not that castle still stands. */
export function castleSlotAt(square: Square): { owner: number; index: number } | null {
  const owner = CASTLE_ROW.indexOf(square.row)
  const index = CASTLE_COLS.indexOf(square.col)
  if (owner < 0 || index < 0) return null
  return { owner, index }
}

export function siegeEngines(state: GameState, player: number): PieceKind[] {
  const kinds: PieceKind[] = []
  for (const row of state.board) {
    for (const piece of row) {
      if (piece && piece.owner === player && isSiege(piece.kind)) kinds.push(piece.kind)
    }
  }
  return kinds
}

function isSiege(kind: PieceKind): boolean {
  return kind === 'trebuchet' || kind === 'batteringRam'
}

const ATTACK_ZONES: Record<PieceKind, [number, number][]> = {
  swordsman: [
    [-1, 0],
    [1, 0],
    [0, -1],
    [0, 1],
  ],
  flail: [
    [-1, -1],
    [-1, 1],
    [1, -1],
    [1, 1],
  ],
  spearman: [
    [-2, 0],
    [2, 0],
    [0, -2],
    [0, 2],
  ],
  archer: [
    [-2, -2],
    [-2, 2],
    [2, -2],
    [2, 2],
  ],
  trebuchet: [
    [-3, 0],
    [3, 0],
    [0, -3],
    [0, 3],
    [-3, -3],
    [-3, 3],
    [3, -3],
    [3, 3],
  ],
  batteringRam: [
    [-1, 0],
    [1, 0],
    [0, -1],
    [0, 1],
  ],
  rock: [],
}

/** A piece's Attack Zone, relative to where it stands, for the rules sheet's
    diagrams. The board asks the engine instead, which knows what is in it. */
export function attackZone(kind: PieceKind): [number, number][] {
  return ATTACK_ZONES[kind]
}

export const PIECE_NAMES: Record<PieceKind, string> = {
  swordsman: 'Swordsman',
  flail: 'Flail',
  spearman: 'Spearman',
  archer: 'Archer',
  trebuchet: 'Trebuchet',
  batteringRam: 'Battering Ram',
  rock: 'Rock',
}

export const PLAYER_NAMES = ['Ivory', 'Umber']

export function lineOf(kind: TokenKind, square: Square): number {
  return kind === 'march' ? square.row : square.col
}
