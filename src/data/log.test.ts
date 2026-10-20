import { describe, expect, it } from 'vitest'
import {
  describeEvent,
  describeMoveOutcome,
  describeSlideOutcome,
  describeThreat,
  describeTurn,
  destroyedSquares,
  lineName,
  squareName,
} from './log'
import { BOARD_COLS, BOARD_ROWS, NEUTRAL } from './types'
import type { GameEvent, GameState, PieceKind, Square } from './types'

describe('board coordinates', () => {
  it('names squares with columns left to right and rows up from Ivory', () => {
    expect(squareName({ row: 0, col: 0 })).toBe('a1')
    expect(squareName({ row: 8, col: 8 })).toBe('i9')
    expect(squareName({ row: 4, col: 4 })).toBe('e5')
  })

  it('names a token line by the kind of line it rides', () => {
    expect(lineName('march', 0)).toBe('row 1')
    expect(lineName('flank', 8)).toBe('column i')
  })
})

describe('describeEvent', () => {
  it('announces an Attack by the line it comes from', () => {
    const event: GameEvent = { type: 'attacked', player: 1, token: 'flank', line: 3 }
    expect(describeEvent(event)).toEqual({ text: 'Attacks along column d', squares: [] })
  })

  it('names a slide by the token and the line it lands on', () => {
    const event: GameEvent = { type: 'slid', player: 0, token: 'march', from: 3, to: 1 }
    expect(describeEvent(event)?.text).toBe('Slides the March token to row 2')
  })

  it('names a move by the token that made it', () => {
    const event: GameEvent = {
      type: 'moved',
      player: 0,
      token: 'flank',
      kind: 'swordsman',
      from: { row: 1, col: 2 },
      to: { row: 3, col: 4 },
    }
    expect(describeEvent(event)).toEqual({
      text: 'Flanks Swordsman c2 → e4',
      squares: [
        { row: 1, col: 2 },
        { row: 3, col: 4 },
      ],
    })
  })

  it('names the attacker and the piece it kills', () => {
    const event: GameEvent = {
      type: 'struck',
      player: 1,
      token: 'flank',
      kind: 'archer',
      from: { row: 3, col: 3 },
      target: { row: 1, col: 1 },
      casualty: { type: 'piece', piece: { kind: 'flail', owner: 0 } },
    }
    expect(describeEvent(event)).toEqual({
      text: 'Archer d4 kills Flail b2',
      squares: [
        { row: 3, col: 3 },
        { row: 1, col: 1 },
      ],
      tone: 'kill',
    })
  })

  it('names a destroyed castle by its square', () => {
    const event: GameEvent = {
      type: 'struck',
      player: 0,
      token: 'march',
      kind: 'trebuchet',
      from: { row: 4, col: 4 },
      target: { row: 7, col: 1 },
      casualty: { type: 'castle' },
    }
    expect(describeEvent(event)?.text).toBe('Trebuchet e5 destroys the castle on b8')
  })

  it('names a killed rock by its square', () => {
    const event: GameEvent = {
      type: 'struck',
      player: 1,
      token: 'flank',
      kind: 'swordsman',
      from: { row: 5, col: 3 },
      target: { row: 4, col: 3 },
      casualty: { type: 'piece', piece: { kind: 'rock', owner: NEUTRAL } },
    }
    expect(describeEvent(event)?.text).toBe('Swordsman d6 kills the rock on d5')
  })

  it('records an attacker that held its fire', () => {
    const event: GameEvent = {
      type: 'heldFire',
      player: 0,
      token: 'march',
      kind: 'swordsman',
      from: { row: 3, col: 3 },
    }
    expect(describeEvent(event)?.text).toBe('Swordsman d4 holds its fire')
  })

  it('names the winner', () => {
    const entry = describeEvent({
      type: 'gameOver',
      outcome: { type: 'winner', player: 1 },
    })
    expect(entry).toEqual({ text: 'Umber wins', squares: [], tone: 'outcome' })
  })

  it('drops turn boundaries, which the log does not show', () => {
    expect(describeEvent({ type: 'turnEnded', player: 0 })).toBeNull()
  })
})

describe('destroyedSquares', () => {
  it('collects the target of every attacker in a batch', () => {
    const events: GameEvent[] = [
      { type: 'slid', player: 0, token: 'march', from: 0, to: 2 },
      { type: 'attacked', player: 0, token: 'march', line: 2 },
      {
        type: 'struck',
        player: 0,
        token: 'march',
        kind: 'swordsman',
        from: { row: 2, col: 1 },
        target: { row: 1, col: 1 },
        casualty: { type: 'piece', piece: { kind: 'flail', owner: 1 } },
      },
      {
        type: 'struck',
        player: 0,
        token: 'march',
        kind: 'batteringRam',
        from: { row: 5, col: 6 },
        target: { row: 6, col: 6 },
        casualty: { type: 'castle' },
      },
    ]
    expect(destroyedSquares(events)).toEqual([
      { row: 1, col: 1 },
      { row: 6, col: 6 },
    ])
  })
})

/** A board with nothing on it but the pieces a test names. */
function position(pieces: [Square, PieceKind, number][]): GameState {
  const board = Array.from({ length: BOARD_ROWS }, () =>
    Array.from({ length: BOARD_COLS }, () => null),
  ) as GameState['board']
  for (const [square, kind, owner] of pieces) board[square.row][square.col] = { kind, owner }
  return {
    board,
    castles: [
      [true, true, true],
      [true, true, true],
    ],
    tokens: [
      [
        { line: 0, side: 'move' },
        { line: 0, side: 'attack' },
      ],
      [
        { line: 6, side: 'move' },
        { line: 6, side: 'attack' },
      ],
    ],
    currentPlayer: 0,
    phase: 'activate',
    pending: ['march', 'flank'],
    pendingLen: 2,
    attackIndex: 0,
    turn: 3,
    outcome: null,
  }
}

describe('describeMoveOutcome', () => {
  const state = position([
    [{ row: 0, col: 1 }, 'spearman', 0],
    [{ row: 1, col: 3 }, 'swordsman', 1],
    [{ row: 4, col: 2 }, 'rock', NEUTRAL],
  ])

  it('names what the Attack after the move would take', () => {
    expect(
      describeMoveOutcome(state, {
        from: { row: 0, col: 1 },
        to: { row: 1, col: 2 },
        threatenedPieces: [
          { row: 1, col: 3 },
          { row: 4, col: 2 },
        ],
        threatenedCastles: [{ row: 7, col: 1 }],
      }),
    ).toBe(
      'Spearman b1 → c2, and your Attack then bears on Swordsman d2, the rock on c5 and the castle on b8.',
    )
  })

  it('says only where the piece goes when nothing follows', () => {
    expect(
      describeMoveOutcome(state, {
        from: { row: 0, col: 1 },
        to: { row: 1, col: 0 },
        threatenedPieces: [],
        threatenedCastles: [],
      }),
    ).toBe('Spearman b1 → a2.')
  })
})

describe('describeSlideOutcome', () => {
  const state = position([[{ row: 2, col: 3 }, 'archer', 1]])

  it('reports both halves of a slide: the move now and the Attack next turn', () => {
    expect(
      describeSlideOutcome(state, {
        token: 'march',
        line: 2,
        movers: [{ row: 2, col: 1 }],
        zone: [{ row: 2, col: 3 }],
        threatenedPieces: [{ row: 2, col: 3 }],
        threatenedCastles: [],
      }),
    ).toBe(
      '1 piece on row 3 could move; next turn the Attack there bears on Archer d3, as the board stands.',
    )
  })

  it('says plainly when a line offers neither', () => {
    expect(
      describeSlideOutcome(state, {
        token: 'flank',
        line: 5,
        movers: [],
        zone: [],
        threatenedPieces: [],
        threatenedCastles: [],
      }),
    ).toBe(
      'Nothing on column f could move; next turn the Attack there bears on nothing, as the board stands.',
    )
  })
})

describe('describeTurn', () => {
  const struck: GameEvent = {
    type: 'struck',
    player: 0,
    token: 'march',
    kind: 'swordsman',
    from: { row: 2, col: 1 },
    target: { row: 3, col: 1 },
    casualty: { type: 'piece', piece: { kind: 'flail', owner: 1 } },
  }

  it('folds an Attack that found no one into one quiet line', () => {
    const entries = describeTurn({
      player: 0,
      round: 0,
      after: null,
      events: [
        { type: 'slid', player: 0, token: 'flank', from: 4, to: 2 },
        { type: 'attacked', player: 0, token: 'march', line: 3 },
        { type: 'passed', player: 0, token: 'flank' },
        { type: 'turnEnded', player: 0 },
      ],
    })
    expect(entries.map((entry) => entry.text)).toEqual([
      'Slides the Flank token to column c',
      'Attacks along row 4, with nothing in reach',
      'Takes no action with the Flank token',
    ])
    expect(entries[1].tone).toBe('quiet')
  })

  it('does not call an Attack empty while its first attacker is still choosing', () => {
    const entries = describeTurn({
      player: 0,
      round: 0,
      after: null,
      events: [{ type: 'attacked', player: 0, token: 'march', line: 2 }],
    })
    expect(entries).toEqual([{ text: 'Attacks along row 3', squares: [] }])
  })

  it('announces an Attack plainly when an attacker answers it', () => {
    const entries = describeTurn({
      player: 0,
      round: 0,
      after: null,
      events: [{ type: 'attacked', player: 0, token: 'march', line: 2 }, struck],
    })
    expect(entries.map((entry) => entry.text)).toEqual([
      'Attacks along row 3',
      'Swordsman b3 kills Flail b4',
    ])
  })
})

describe('describeThreat', () => {
  const state = position([
    [{ row: 1, col: 4 }, 'batteringRam', 0],
    [{ row: 2, col: 4 }, 'batteringRam', 1],
    [{ row: 2, col: 2 }, 'swordsman', 0],
  ])

  it('names every attacker that reaches the square', () => {
    expect(
      describeThreat(
        state,
        { row: 2, col: 2 },
        [
          { from: { row: 2, col: 3 }, kind: 'swordsman', targets: [{ row: 2, col: 2 }] },
          { from: { row: 4, col: 2 }, kind: 'spearman', targets: [{ row: 2, col: 2 }] },
        ],
        "Umber's Attack along row 3 next turn",
      ),
    ).toBe("Umber's Attack along row 3 next turn: Swordsman d3 and Spearman c5 can kill this swordsman.")
  })

  it('speaks of a castle when a siege engine is the attacker', () => {
    expect(
      describeThreat(
        state,
        { row: 1, col: 4 },
        [{ from: { row: 2, col: 4 }, kind: 'batteringRam', targets: [{ row: 1, col: 4 }] }],
        "Umber's Attack along column e next turn",
      ),
    ).toBe("Umber's Attack along column e next turn: Battering Ram e3 can destroy this castle.")
  })

  it('says nothing when no attacker reaches the square', () => {
    expect(describeThreat(state, { row: 5, col: 5 }, [], 'Your Attack')).toBeNull()
  })
})
