// Turns engine events into the sentences shown in the sidebar.

import { COLUMN_LETTERS, PIECE_NAMES, PLAYER_NAMES, TOKEN_NAMES, pieceAt } from './types'
import type {
  Casualty,
  GameEvent,
  GameState,
  MoveOutcome,
  Piece,
  SlideOutcome,
  Square,
  Threat,
  TokenKind,
} from './types'

export interface LogEntry {
  player: number
  text: string
  /** Set on entries that removed something, so the log can mark them. */
  grave?: boolean
}

/** Columns run a-i and rows 1-7 up from Ivory's back row, as the rules sheet
    draws the board. */
export function squareName(square: Square): string {
  return `${COLUMN_LETTERS[square.col]}${square.row + 1}`
}

export function lineName(kind: TokenKind, line: number): string {
  return kind === 'march' ? `row ${line + 1}` : `column ${COLUMN_LETTERS[line]}`
}

function pieceName(piece: Piece, square: Square): string {
  return piece.kind === 'rock'
    ? `the rock on ${squareName(square)}`
    : `${PIECE_NAMES[piece.kind]} ${squareName(square)}`
}

function describeCasualty(casualty: Casualty, target: Square): string {
  return casualty.type === 'castle'
    ? `destroys the castle on ${squareName(target)}`
    : `kills ${pieceName(casualty.piece, target)}`
}

export function describeEvent(event: GameEvent): LogEntry | null {
  switch (event.type) {
    case 'slid':
      return {
        player: event.player,
        text: `slides the ${TOKEN_NAMES[event.token]} token to ${lineName(event.token, event.to)}`,
      }
    case 'moved': {
      const verb = event.token === 'march' ? 'marches' : 'flanks'
      return {
        player: event.player,
        text: `${verb} ${PIECE_NAMES[event.kind]} ${squareName(event.from)} → ${squareName(event.to)}`,
      }
    }
    case 'passed':
      return {
        player: event.player,
        text: `takes no action with the ${TOKEN_NAMES[event.token]} token`,
      }
    case 'attacked':
      return {
        player: event.player,
        text: `attacks along ${lineName(event.token, event.line)}`,
      }
    case 'struck':
      return {
        player: event.player,
        text: `${PIECE_NAMES[event.kind]} ${squareName(event.from)} ${describeCasualty(event.casualty, event.target)}`,
        grave: true,
      }
    case 'heldFire':
      return {
        player: event.player,
        text: `${PIECE_NAMES[event.kind]} ${squareName(event.from)} holds its fire`,
      }
    case 'gameOver':
      // No speaker prefix: the sentence already names the winner.
      return {
        player: -1,
        text:
          event.outcome.type === 'winner'
            ? `${PLAYER_NAMES[event.outcome.player]} wins`
            : 'the campaign ends in a draw',
        grave: true,
      }
    case 'turnEnded':
      return null
  }
}

/** Squares emptied by these events, for the strike flash on the board. */
export function destroyedSquares(events: GameEvent[]): Square[] {
  return events.filter((event) => event.type === 'struck').map((event) => event.target)
}

function joinList(items: string[]): string {
  if (items.length < 2) return items.join('')
  return `${items.slice(0, -1).join(', ')} and ${items[items.length - 1]}`
}

/** Names what stands on each square, for a sentence about attacking it. */
function targetNames(state: GameState, threat: Threat): string[] {
  return [
    ...threat.threatenedPieces.map((square) => {
      const piece = pieceAt(state, square)
      return piece ? pieceName(piece, square) : squareName(square)
    }),
    ...threat.threatenedCastles.map((square) => `the castle on ${squareName(square)}`),
  ]
}

/** The sentence under the board while a destination is under the cursor. */
export function describeMoveOutcome(state: GameState, outcome: MoveOutcome): string {
  const journey = `${squareName(outcome.from)} → ${squareName(outcome.to)}`
  const targets = targetNames(state, outcome)
  if (targets.length === 0) return `${journey}.`
  return `${journey}, and your Attack then bears on ${joinList(targets)}.`
}

/** The sentence under the board while a slide destination is under the cursor. */
export function describeSlideOutcome(state: GameState, outcome: SlideOutcome): string {
  const where = lineName(outcome.token, outcome.line)
  const movers =
    outcome.movers.length === 0
      ? `Nothing on ${where} could move`
      : `${outcome.movers.length} piece${outcome.movers.length === 1 ? '' : 's'} on ${where} could move`
  const targets = targetNames(state, outcome)
  const attack =
    targets.length === 0
      ? 'next turn the Attack there bears on nothing, as the board stands'
      : `next turn the Attack there bears on ${joinList(targets)}, as the board stands`
  return `${movers}; ${attack}.`
}
