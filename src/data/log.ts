// Turns engine events and previews into the sentences the interface shows.

import { COLUMN_LETTERS, PIECE_NAMES, PLAYER_NAMES, TOKEN_NAMES, pieceAt } from './types'
import type {
  Casualty,
  GameEvent,
  GameState,
  MoveOutcome,
  Piece,
  Shot,
  SlideOutcome,
  Square,
  Threat,
  TokenKind,
} from './types'
import type { Turn } from './turns'

export interface LogEntry {
  text: string
  /** The squares the entry is about, lit on the board while it is hovered. */
  squares: Square[]
  /** A kill or a destroyed castle, an Attack that found nothing, or the end. */
  tone?: 'kill' | 'quiet' | 'outcome'
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
        text: `Slides the ${TOKEN_NAMES[event.token]} token to ${lineName(event.token, event.to)}`,
        squares: [],
      }
    case 'moved': {
      const verb = event.token === 'march' ? 'Marches' : 'Flanks'
      return {
        text: `${verb} ${PIECE_NAMES[event.kind]} ${squareName(event.from)} → ${squareName(event.to)}`,
        squares: [event.from, event.to],
      }
    }
    case 'passed':
      return { text: `Takes no action with the ${TOKEN_NAMES[event.token]} token`, squares: [] }
    case 'attacked':
      return { text: `Attacks along ${lineName(event.token, event.line)}`, squares: [] }
    case 'struck':
      return {
        text: `${PIECE_NAMES[event.kind]} ${squareName(event.from)} ${describeCasualty(event.casualty, event.target)}`,
        squares: [event.from, event.target],
        tone: 'kill',
      }
    case 'heldFire':
      return {
        text: `${PIECE_NAMES[event.kind]} ${squareName(event.from)} holds its fire`,
        squares: [event.from],
      }
    case 'gameOver':
      return {
        text:
          event.outcome.type === 'winner'
            ? `${PLAYER_NAMES[event.outcome.player]} wins`
            : 'The game ends in a draw',
        squares: [],
        tone: 'outcome',
      }
    case 'turnEnded':
      return null
  }
}

/** A turn's dispatches. An Attack that found no one to ask is folded into one
    quiet line rather than announced as though something followed. One still
    waiting on its first attacker has simply not been answered yet. */
export function describeTurn(turn: Turn): LogEntry[] {
  const entries: LogEntry[] = []
  turn.events.forEach((event, index) => {
    const entry = describeEvent(event)
    if (!entry) return
    if (event.type === 'attacked') {
      const next = turn.events[index + 1]
      const waiting = next === undefined && turn.after === null
      const answered = next?.type === 'struck' || next?.type === 'heldFire'
      if (!answered && !waiting) {
        entries.push({ ...entry, text: `${entry.text}, with nothing in reach`, tone: 'quiet' })
        return
      }
    }
    entries.push(entry)
  })
  return entries
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
  const piece = pieceAt(state, outcome.from)
  const name = piece ? `${PIECE_NAMES[piece.kind]} ` : ''
  const journey = `${name}${squareName(outcome.from)} → ${squareName(outcome.to)}`
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

/** Who in an Attack can take what stands on `square`, as one sentence:
    "Umber's Attack along column e next turn: Battering Ram e6 can destroy this
    castle." Null when no attacker reaches it. */
export function describeThreat(
  state: GameState,
  square: Square,
  shots: Shot[],
  attack: string,
): string | null {
  const attackers = shots.filter((shot) =>
    shot.targets.some((target) => target.row === square.row && target.col === square.col),
  )
  if (attackers.length === 0) return null
  const names = joinList(attackers.map((shot) => `${PIECE_NAMES[shot.kind]} ${squareName(shot.from)}`))
  const piece = pieceAt(state, square)
  const castle = attackers[0].kind === 'trebuchet' || attackers[0].kind === 'batteringRam'
  const object = castle
    ? 'destroy this castle'
    : `kill this ${piece ? PIECE_NAMES[piece.kind].toLowerCase() : 'piece'}`
  return `${attack}: ${names} can ${object}.`
}
