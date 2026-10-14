<script lang="ts">
  import Ornament from './Ornament.svelte'
  import PieceIcon from './PieceIcon.svelte'
  import RulesOverlay from './RulesOverlay.svelte'
  import { DIFFICULTIES } from '../ai/aiController'
  import { PLAYER_NAMES } from '../data/types'
  import type { GameSetup, SavedGame } from '../data/session'

  interface Props {
    /** An unfinished game, offered back before a new one is begun. */
    saved: SavedGame | null
    onstart: (setup: GameSetup) => void
    onresume: (saved: SavedGame) => void
  }

  let { saved, onstart, onresume }: Props = $props()

  type Mode = 'solo' | 'hotseat' | 'watch'
  const MODES: { mode: Mode; label: string }[] = [
    { mode: 'solo', label: 'Play the machine' },
    { mode: 'hotseat', label: 'Two players' },
    { mode: 'watch', label: 'Watch two machines' },
  ]

  let mode: Mode = $state('solo')
  let side = $state(0)
  let level = $state(1)
  let showRules = $state(false)

  function start() {
    const difficulty = DIFFICULTIES[level]
    const seats: ('human' | 'ai')[] =
      mode === 'hotseat'
        ? ['human', 'human']
        : mode === 'watch'
          ? ['ai', 'ai']
          : side === 0
            ? ['human', 'ai']
            : ['ai', 'human']
    onstart({ seats, iterations: difficulty.iterations, difficulty: difficulty.name })
  }

  function describe(save: SavedGame): string {
    const humans = save.setup.seats.filter((seat) => seat === 'human').length
    const against =
      humans === 2
        ? 'two players'
        : humans === 0
          ? `two machines at ${save.setup.difficulty}`
          : `as ${PLAYER_NAMES[save.setup.seats.indexOf('human')]} against ${save.setup.difficulty}`
    return `Turn ${save.game.turn + 1}, ${against}`
  }
</script>

<div class="menu">
  <div class="card panel">
    <header class="title">
      <Ornament side="left" />
      <h1>Assedio</h1>
      <Ornament side="right" />
    </header>
    <p class="tagline">
      Two command tokens, one board, and a siege that turns on where you point them.
    </p>

    {#if saved}
      <section class="resume">
        <span class="engine"><PieceIcon kind="trebuchet" owner={0} /></span>
        <span class="what">
          <strong>A siege in progress</strong>
          <span>{describe(saved)}</span>
        </span>
        <button class="primary" onclick={() => onresume(saved)}>Resume</button>
      </section>
    {/if}

    <fieldset>
      <legend class="small-caps">Opponent</legend>
      <div class="segments three">
        {#each MODES as option (option.mode)}
          <label class:picked={mode === option.mode}>
            <input type="radio" bind:group={mode} value={option.mode} />
            {option.label}
          </label>
        {/each}
      </div>
    </fieldset>

    {#if mode === 'solo'}
      <fieldset>
        <legend class="small-caps">Your side</legend>
        <div class="segments">
          {#each PLAYER_NAMES as sideName, index (sideName)}
            <label class:picked={side === index}>
              <input type="radio" bind:group={side} value={index} />
              <span class={`disc side-${index}`}></span>
              {sideName}{index === 0 ? ' · moves first' : ''}
            </label>
          {/each}
        </div>
      </fieldset>
    {/if}

    {#if mode !== 'hotseat'}
      <fieldset>
        <legend class="small-caps">Strength</legend>
        <div class="segments four">
          {#each DIFFICULTIES as difficulty, index (difficulty.name)}
            <label class:picked={level === index}>
              <input type="radio" bind:group={level} value={index} />
              {difficulty.name}
            </label>
          {/each}
        </div>
        <p class="blurb">{DIFFICULTIES[level].blurb}</p>
      </fieldset>
    {/if}

    <div class="actions">
      <button class:primary={!saved} onclick={start}>
        {saved ? 'Begin a new siege' : 'Begin the siege'}
      </button>
      <button onclick={() => (showRules = true)}>How to play</button>
    </div>
    {#if saved}
      <p class="note">A new siege replaces the one in progress.</p>
    {/if}
  </div>
</div>

{#if showRules}
  <RulesOverlay onclose={() => (showRules = false)} />
{/if}

<style>
  .menu {
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: 1rem;
  }

  .card > * {
    min-width: 0;
  }

  .card {
    width: min(34rem, 100%);
    padding: 1.3rem 1.5rem 1.4rem;
    display: grid;
    gap: 0.9rem;
    text-align: center;
  }

  .title {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    font-size: 1.9rem;
    --rule: 2.4em;
  }

  h1 {
    font-size: 2.6rem;
    font-weight: 900;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .tagline {
    color: var(--ink-soft);
    font-style: italic;
    margin-top: -0.4rem;
  }

  .resume {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.8rem;
    border: 1.5px solid var(--copper);
    border-radius: 10px;
    background: rgba(248, 211, 157, 0.35);
    text-align: left;
  }

  .resume .engine {
    width: 2.4rem;
    height: 2.4rem;
  }

  .what {
    display: grid;
    line-height: 1.25;
  }

  .what span {
    font-size: 0.88rem;
    color: var(--ink-soft);
  }

  fieldset {
    border: none;
    display: grid;
    gap: 0.35rem;
    text-align: left;
  }

  .segments {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    border: 1.5px solid var(--copper);
    border-radius: 9px;
    overflow: hidden;
  }

  label {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.5rem 0.4rem;
    font-weight: 700;
    font-size: 0.92rem;
    text-align: center;
    cursor: pointer;
    background: var(--panel);
    transition: background 0.15s;
    line-height: 1.2;
  }

  label + label {
    border-left: 1px solid var(--copper-soft);
  }

  label:hover {
    background: var(--parchment-deep);
  }

  label.picked {
    background: var(--navy);
    color: var(--peach);
  }

  label:has(input:focus-visible) {
    outline: 2.5px solid var(--march-deep);
    outline-offset: -3px;
  }

  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .disc {
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 50%;
    flex: none;
  }

  .disc.side-0 {
    background: var(--ivory);
    box-shadow: 0 0 0 2px var(--ivory-rim);
  }

  .disc.side-1 {
    background: var(--umber);
    box-shadow: 0 0 0 2px var(--umber-rim);
  }

  .blurb {
    font-size: 0.88rem;
    font-style: italic;
    color: var(--ink-soft);
    text-align: center;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: center;
    margin-top: 0.3rem;
  }

  .actions button {
    font-size: 1.02rem;
    padding: 0.55rem 1.2rem;
  }

  .note {
    font-size: 0.82rem;
    color: var(--ink-faint);
    margin-top: -0.4rem;
  }

  @media (max-width: 480px) {
    .title {
      font-size: 1.5rem;
    }

    h1 {
      font-size: 1.9rem;
    }

    .segments.three,
    .segments.four {
      grid-auto-flow: row;
      grid-template-columns: 1fr 1fr;
    }

    .segments.three label:last-child {
      grid-column: 1 / -1;
    }

    label + label {
      border-left: none;
    }

    .segments label {
      border-top: 1px solid var(--copper-soft);
    }
  }
</style>
