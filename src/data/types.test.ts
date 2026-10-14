import { describe, expect, it } from 'vitest'
import { castleSlotAt, isHilltop, squareKey, zoneSquares } from './types'
import type { GameState } from './types'

/** Only the castles matter to the terrain rules under test. */
function board(castles: boolean[][]): GameState {
  return { castles } as GameState
}

describe('attack zones', () => {
  it('matches the printed diagrams', () => {
    const centre = { row: 3, col: 3 }
    const keys = (kind: Parameters<typeof zoneSquares>[1]) =>
      zoneSquares(centre, kind).map(squareKey).sort()

    expect(keys('swordsman')).toEqual(['2,3', '3,2', '3,4', '4,3'].sort())
    expect(keys('flail')).toEqual(['2,2', '2,4', '4,2', '4,4'].sort())
    expect(keys('spearman')).toEqual(['1,3', '3,1', '3,5', '5,3'].sort())
    expect(keys('archer')).toEqual(['1,1', '1,5', '5,1', '5,5'].sort())
  })

  it('gives a rock no zone at all', () => {
    expect(zoneSquares({ row: 3, col: 3 }, 'rock')).toEqual([])
  })

  it('clips targets that fall off the board', () => {
    expect(zoneSquares({ row: 0, col: 0 }, 'archer').map(squareKey)).toEqual(['2,2'])
    // Nine columns, so col 8 is the last one.
    expect(zoneSquares({ row: 3, col: 8 }, 'archer').map(squareKey).sort()).toEqual(
      ['1,6', '5,6'].sort(),
    )
  })

  it('gives the centre trebuchet a line on every castle', () => {
    const castles = zoneSquares({ row: 3, col: 4 }, 'trebuchet')
      .filter((square) => castleSlotAt(square) !== null)
      .map(squareKey)
      .sort()
    expect(castles).toEqual(['0,1', '0,4', '0,7', '6,1', '6,4', '6,7'])
  })
})

describe('terrain', () => {
  it('puts the printed hilltops on the middle row only', () => {
    const intact = board([
      [true, true, true],
      [true, true, true],
    ])
    expect(isHilltop(intact, { row: 3, col: 1 })).toBe(true)
    expect(isHilltop(intact, { row: 3, col: 4 })).toBe(true)
    expect(isHilltop(intact, { row: 3, col: 7 })).toBe(true)
    expect(isHilltop(intact, { row: 0, col: 1 })).toBe(false)
    expect(isHilltop(intact, { row: 3, col: 2 })).toBe(false)
  })

  it('turns every destroyed castle into a hilltop', () => {
    const destroyed = board([
      [false, true, true],
      [true, true, false],
    ])
    expect(isHilltop(destroyed, { row: 0, col: 1 })).toBe(true)
    expect(isHilltop(destroyed, { row: 6, col: 7 })).toBe(true)
    expect(isHilltop(destroyed, { row: 0, col: 4 })).toBe(false)
  })

  it('finds a castle slot on the three back-row castle columns', () => {
    expect(castleSlotAt({ row: 0, col: 4 })).toEqual({ owner: 0, index: 1 })
    expect(castleSlotAt({ row: 6, col: 7 })).toEqual({ owner: 1, index: 2 })
    expect(castleSlotAt({ row: 0, col: 2 })).toBeNull()
    expect(castleSlotAt({ row: 0, col: 0 })).toBeNull()
    expect(castleSlotAt({ row: 3, col: 4 })).toBeNull()
  })
})
