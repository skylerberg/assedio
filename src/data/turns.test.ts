import { describe, expect, it } from 'vitest'
import { advanceIds, idsAfter, initialIds, recordTurns, summarize, turnMarks } from './turns'
import { BOARD_COLS, BOARD_ROWS, NEUTRAL } from './types'
import type { GameEvent, GameState } from './types'

function stateWith(pieces: [number, number][]): GameState {
  const board = Array.from({ length: BOARD_ROWS }, () =>
    Array.from({ length: BOARD_COLS }, () => null),
  ) as GameState['board']
  for (const [row, col] of pieces) board[row][col] = { kind: 'swordsman', owner: 0 }
  return { board, turn: 0 } as GameState
}

const slid: GameEvent = { type: 'slid', player: 0, token: 'flank', from: 4, to: 2 }
const moved: GameEvent = {
  type: 'moved',
  player: 0,
  token: 'flank',
  kind: 'swordsman',
  from: { row: 1, col: 2 },
  to: { row: 2, col: 3 },
}
const killed: GameEvent = {
  type: 'struck',
  player: 0,
  token: 'march',
  kind: 'swordsman',
  from: { row: 2, col: 3 },
  target: { row: 3, col: 3 },
  casualty: { type: 'piece', piece: { kind: 'rock', owner: NEUTRAL } },
}
const razed: GameEvent = {
  type: 'struck',
  player: 1,
  token: 'march',
  kind: 'batteringRam',
  from: { row: 2, col: 4 },
  target: { row: 1, col: 4 },
  casualty: { type: 'castle' },
}
const ended = (player: number): GameEvent => ({ type: 'turnEnded', player })

describe('recordTurns', () => {
  const after = stateWith([])

  it('opens a turn on its first event and closes it when it ends', () => {
    let turns = recordTurns([], [slid], 0, after)
    expect(turns).toHaveLength(1)
    expect(turns[0].after).toBeNull()

    turns = recordTurns(turns, [moved, ended(0)], 0, after)
    expect(turns).toHaveLength(1)
    expect(turns[0].events).toEqual([slid, moved, ended(0)])
    expect(turns[0].after).toBe(after)
  })

  it('starts the next side on a fresh turn, in the round it was played', () => {
    const first = recordTurns([], [slid, ended(0)], 0, after)
    const second = recordTurns(first, [{ ...slid, player: 1 }], 0, after)
    expect(second.map((turn) => turn.player)).toEqual([0, 1])
    expect(first).toHaveLength(1)
  })

  it('closes the winning turn on the game ending, with no turn boundary', () => {
    const turns = recordTurns(
      [],
      [razed, { type: 'gameOver', outcome: { type: 'winner', player: 1 } }],
      3,
      after,
    )
    expect(turns[0].after).toBe(after)
    expect(turns[0].round).toBe(3)
  })

  it('files a draw called after the last turn ended under that turn', () => {
    const turns = recordTurns([], [slid, ended(0), { type: 'gameOver', outcome: { type: 'draw' } }], 0, after)
    expect(turns).toHaveLength(1)
    expect(turns[0].events.at(-1)?.type).toBe('gameOver')
  })
})

describe('turnMarks', () => {
  it('collects the slide, the moves and who lost what', () => {
    const marks = turnMarks({ player: 0, round: 0, after: null, events: [slid, moved, killed] })
    expect(marks.slide).toEqual({ token: 'flank', from: 4, to: 2 })
    expect(marks.moves).toEqual([{ from: moved.from, to: moved.to }])
    expect(marks.kills).toEqual([
      { square: { row: 3, col: 3 }, casualty: killed.casualty, victim: NEUTRAL },
    ])
  })

  it('charges a destroyed castle to the side that owned it', () => {
    const marks = turnMarks({ player: 1, round: 0, after: null, events: [razed] })
    expect(marks.kills[0].victim).toBe(0)
  })
})

describe('summarize', () => {
  it('counts castles, pieces and rocks by the side that took them', () => {
    const summary = summarize([
      { player: 0, round: 0, after: null, events: [killed] },
      { player: 1, round: 2, after: null, events: [razed] },
    ])
    expect(summary).toEqual({ rounds: 3, castles: [0, 1], pieces: [0, 0], rocks: [1, 0] })
  })
})

describe('piece ids', () => {
  const start = stateWith([
    [1, 2],
    [3, 3],
  ])

  it('numbers the pieces in board order', () => {
    const ids = initialIds(start)
    expect(ids[1 * BOARD_COLS + 2]).toBe(1)
    expect(ids[3 * BOARD_COLS + 3]).toBe(2)
    expect(ids.filter((id) => id !== 0)).toHaveLength(2)
  })

  it('carries an id along a move and drops it on a kill', () => {
    const ids = advanceIds(initialIds(start), [moved, killed])
    expect(ids[2 * BOARD_COLS + 3]).toBe(1)
    expect(ids[1 * BOARD_COLS + 2]).toBe(0)
    expect(ids[3 * BOARD_COLS + 3]).toBe(0)
  })

  it('replays a prefix of the game to the same ids', () => {
    const turns = [{ player: 0, round: 0, after: null, events: [moved] }]
    expect(idsAfter(start, turns, 1)).toEqual(advanceIds(initialIds(start), [moved]))
    expect(idsAfter(start, turns, 0)).toEqual(initialIds(start))
  })
})
