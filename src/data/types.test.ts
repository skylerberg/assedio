import { describe, expect, it } from 'vitest'
import { attackZone, castleSlotAt, isHilltop } from './types'
import type { GameState } from './types'

/** Only the castles matter to the terrain rules under test. */
function board(castles: boolean[][]): GameState {
  return { castles } as GameState
}

describe('attack zones', () => {
  const offsets = (kind: Parameters<typeof attackZone>[0]) =>
    attackZone(kind)
      .map(([row, col]) => `${row},${col}`)
      .sort()

  it('matches the printed diagrams', () => {
    expect(offsets('swordsman')).toEqual(['-1,0', '0,-1', '0,1', '1,0'])
    expect(offsets('flail')).toEqual(['-1,-1', '-1,1', '1,-1', '1,1'])
    expect(offsets('spearman')).toEqual(['-2,0', '0,-2', '0,2', '2,0'])
    expect(offsets('archer')).toEqual(['-2,-2', '-2,2', '2,-2', '2,2'])
  })

  it('gives a rock no zone at all', () => {
    expect(attackZone('rock')).toEqual([])
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
