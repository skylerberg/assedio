<script lang="ts">
  import GameScreen from './components/GameScreen.svelte'
  import MainMenu from './components/MainMenu.svelte'
  import { loadGame } from './data/session'
  import type { GameSetup, SavedGame } from './data/session'
  import { initEngine } from './engine/wasmEngine'

  let ready = $state(false)
  let failure: string | null = $state(null)
  /** The game on screen, and the save it picks up from when it is resumed. */
  let playing: { setup: GameSetup; saved: SavedGame | null } | null = $state(null)
  let saved: SavedGame | null = $state(unfinished())

  function unfinished(): SavedGame | null {
    const save = loadGame()
    return save && !save.game.outcome ? save : null
  }

  initEngine().then(
    () => (ready = true),
    (error) => (failure = String(error)),
  )
</script>

{#if failure}
  <div class="notice">
    <h1>The engine failed to load</h1>
    <p>{failure}</p>
    <p class="hint">Run <code>pnpm run build:wasm</code> and reload.</p>
  </div>
{:else if !ready}
  <div class="notice">
    <p>Mustering…</p>
  </div>
{:else if playing}
  {#key playing}
    <GameScreen
      setup={playing.setup}
      saved={playing.saved}
      onexit={() => {
        playing = null
        saved = unfinished()
      }}
    />
  {/key}
{:else}
  <MainMenu
    {saved}
    onstart={(setup) => (playing = { setup, saved: null })}
    onresume={(save) => (playing = { setup: save.setup, saved: save })}
  />
{/if}

<style>
  .notice {
    min-height: 100dvh;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 0.5rem;
    text-align: center;
    padding: 2rem;
    color: var(--ink-soft);
    font-style: italic;
  }

  .hint {
    font-size: 0.9rem;
  }

  code {
    font-family: ui-monospace, monospace;
    font-style: normal;
    background: var(--parchment-deep);
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
  }
</style>
