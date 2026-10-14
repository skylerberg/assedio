// The game as a sequence of turns: what each side did, the board it left, and
// which piece is which, so the interface can replay, animate and sum it up.

import { BOARD_COLS, BOARD_ROWS, NEUTRAL } from './types'
import type { Casualty, GameEvent, GameState, Square, TokenKind } from './types'

export interface Turn {
  player: number
  /** The round it was played in, as `GameState.turn` counts them. */
  round: number
  events: GameEvent[]
  /** The position the turn ended on; null while it is still being played. */
  after: GameState | null
}

/** Files each batch of events under the turn it belongs to. `round` and
    `state` are the round the batch was played in and the position it left. */
export function recordTurns(
  turns: Turn[],
  events: GameEvent[],
  round: number,
  state: GameState,
): Turn[] {
  const result = turns.slice()
  for (const event of events) {
    let open = result.at(-1)
    // A draw is called once the last turn has already ended; it belongs to it.
    if (open && event.type === 'gameOver') {
      result[result.length - 1] = { ...open, events: [...open.events, event], after: state }
      continue
    }
    if (!open || open.after) {
      if (event.type === 'gameOver') continue
      open = { player: event.player, round, events: [], after: null }
      result.push(open)
    }
    const closes = event.type === 'turnEnded'
    result[result.length - 1] = {
      ...open,
      events: [...open.events, event],
      after: closes ? state : null,
    }
  }
  return result
}

/** What a turn did, for the marks the board leaves until the next one. */
export interface TurnMarks {
  player: number
  slide: { token: TokenKind; from: number; to: number } | null
  moves: { from: Square; to: Square }[]
  kills: { square: Square; casualty: Casualty; victim: number }[]
}

export function turnMarks(turn: Turn): TurnMarks {
  const marks: TurnMarks = { player: turn.player, slide: null, moves: [], kills: [] }
  for (const event of turn.events) {
    if (event.type === 'slid') {
      marks.slide = { token: event.token, from: event.from, to: event.to }
    } else if (event.type === 'moved') {
      marks.moves.push({ from: event.from, to: event.to })
    } else if (event.type === 'struck') {
      const victim =
        event.casualty.type === 'piece' ? event.casualty.piece.owner : 1 - event.player
      marks.kills.push({ square: event.target, casualty: event.casualty, victim })
    }
  }
  return marks
}

/** What each side achieved over the game, indexed by player. */
export interface Summary {
  /** Rounds begun, as the banner counts them: a turn each makes one. */
  rounds: number
  castles: number[]
  pieces: number[]
  rocks: number[]
}

export function summarize(turns: Turn[]): Summary {
  const summary: Summary = {
    rounds: (turns.at(-1)?.round ?? -1) + 1,
    castles: [0, 0],
    pieces: [0, 0],
    rocks: [0, 0],
  }
  for (const turn of turns) {
    for (const event of turn.events) {
      if (event.type !== 'struck') continue
      if (event.casualty.type === 'castle') summary.castles[event.player] += 1
      else if (event.casualty.piece.owner === NEUTRAL) summary.rocks[event.player] += 1
      else summary.pieces[event.player] += 1
    }
  }
  return summary
}

/** A stable id for every piece, by square, so a move can be drawn as the same
    piece travelling rather than one vanishing and another appearing. Zero is an
    empty square. */
export type PieceIds = number[]

export function squareIndex(square: Square): number {
  return square.row * BOARD_COLS + square.col
}

export function initialIds(state: GameState): PieceIds {
  const ids: PieceIds = []
  let next = 1
  for (let row = 0; row < BOARD_ROWS; row++) {
    for (let col = 0; col < BOARD_COLS; col++) {
      ids.push(state.board[row][col] ? next++ : 0)
    }
  }
  return ids
}

export function advanceIds(ids: PieceIds, events: GameEvent[]): PieceIds {
  const result = ids.slice()
  for (const event of events) {
    if (event.type === 'moved') {
      result[squareIndex(event.to)] = result[squareIndex(event.from)]
      result[squareIndex(event.from)] = 0
    } else if (event.type === 'struck' && event.casualty.type === 'piece') {
      result[squareIndex(event.target)] = 0
    }
  }
  return result
}

/** The ids as they stood after the first `count` turns, replayed from the
    opening so a reviewed position keeps every piece's identity. */
export function idsAfter(start: GameState, turns: Turn[], count: number): PieceIds {
  let ids = initialIds(start)
  for (const turn of turns.slice(0, count)) ids = advanceIds(ids, turn.events)
  return ids
}
