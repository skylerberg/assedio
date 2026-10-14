import { describe, expect, it } from 'vitest'
import { parseSave } from './session'
import { BOARD_COLS, BOARD_ROWS } from './types'

const valid = {
  version: 1,
  setup: { seats: ['human', 'ai'], iterations: 30_000, difficulty: 'Knight' },
  game: { board: Array.from({ length: BOARD_ROWS }, () => []), turn: 4 },
  turns: [],
  ids: Array.from({ length: BOARD_ROWS * BOARD_COLS }, () => 0),
}

describe('parseSave', () => {
  it('reads back a save of the current shape', () => {
    const save = parseSave(valid)
    expect(save?.setup.difficulty).toBe('Knight')
    expect(save?.game.turn).toBe(4)
  })

  it('drops a save from another version rather than misreading it', () => {
    expect(parseSave({ ...valid, version: 0 })).toBeNull()
  })

  it('drops anything that is not a save', () => {
    expect(parseSave(null)).toBeNull()
    expect(parseSave('a game')).toBeNull()
    expect(parseSave({ ...valid, ids: [] })).toBeNull()
    expect(parseSave({ ...valid, game: { board: [] } })).toBeNull()
  })
})
