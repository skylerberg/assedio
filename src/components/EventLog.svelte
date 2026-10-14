<script lang="ts">
  import { describeTurn } from '../data/log'
  import { PLAYER_NAMES } from '../data/types'
  import type { Square } from '../data/types'
  import type { Turn } from '../data/turns'

  interface Props {
    turns: Turn[]
    /** Light the squares a dispatch is about; empty to clear. */
    onhighlight: (squares: Square[]) => void
    /** Open the review at a turn, once the game allows one. */
    onreview?: (count: number) => void
    /** The turn on the board while reviewing, counted from one. */
    reviewing?: number | null
  }

  let { turns, onhighlight, onreview, reviewing = null }: Props = $props()
  let list: HTMLOListElement | null = $state(null)

  $effect(() => {
    void turns.length
    void turns.at(-1)?.events.length
    if (list && reviewing === null) list.scrollTop = list.scrollHeight
  })
</script>

<section class="panel log">
  <h2 class="head">Dispatches</h2>
  <ol bind:this={list}>
    {#each turns as turn, index (index)}
      <li class="turn" class:current={reviewing === index + 1}>
        {#if onreview}
          <button class="turn-head" onclick={() => onreview(index + 1)}>
            <span class={`disc side-${turn.player}`}></span>
            Turn {turn.round + 1} · {PLAYER_NAMES[turn.player]}
          </button>
        {:else}
          <div class="turn-head">
            <span class={`disc side-${turn.player}`}></span>
            Turn {turn.round + 1} · {PLAYER_NAMES[turn.player]}
          </div>
        {/if}
        <ul>
          {#each describeTurn(turn) as entry, i (i)}
            <li class={entry.tone ?? ''}>
              {#if entry.squares.length > 0}
                <button
                  class="entry"
                  onmouseenter={() => onhighlight(entry.squares)}
                  onmouseleave={() => onhighlight([])}
                  onfocus={() => onhighlight(entry.squares)}
                  onblur={() => onhighlight([])}
                >
                  {entry.text}
                </button>
              {:else}
                {entry.text}
              {/if}
            </li>
          {/each}
        </ul>
      </li>
    {:else}
      <li class="empty">No moves yet. Ivory moves first.</li>
    {/each}
  </ol>
</section>

<style>
  .log {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  ol {
    list-style: none;
    overflow-y: auto;
    flex: 1;
    min-height: 6rem;
    padding: 0.5rem 0.75rem 0.7rem;
    display: grid;
    gap: 0.55rem;
    align-content: start;
    font-size: 0.86rem;
    line-height: 1.3;
  }

  .turn.current {
    background: rgba(185, 120, 42, 0.14);
    border-radius: 6px;
    margin: 0 -0.35rem;
    padding: 0 0.35rem;
  }

  .turn-head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
    font-family: var(--font-display);
    font-size: 0.78rem;
    font-weight: 800;
    letter-spacing: 0.04em;
    color: var(--ink);
    background: none;
    border: none;
    border-bottom: 1px solid var(--copper-soft);
    border-radius: 0;
    padding: 0 0 0.15rem;
    text-align: left;
  }

  button.turn-head:hover:not(:disabled) {
    background: none;
    color: var(--copper-deep);
  }

  .disc {
    width: 0.65rem;
    height: 0.65rem;
    border-radius: 50%;
    flex: none;
  }

  .disc.side-0 {
    background: var(--ivory);
    box-shadow: 0 0 0 1.5px var(--ivory-rim);
  }

  .disc.side-1 {
    background: var(--umber);
    box-shadow: 0 0 0 1.5px var(--umber-rim);
  }

  ul {
    list-style: none;
    display: grid;
    gap: 0.1rem;
    padding: 0.2rem 0 0 1.05rem;
    color: var(--ink-soft);
  }

  li.kill {
    color: var(--ink);
    font-weight: 700;
  }

  li.kill::before {
    content: '✕ ';
    color: var(--attack-deep);
  }

  li.quiet {
    font-style: italic;
  }

  li.outcome {
    color: var(--attack-deep);
    font-weight: 800;
  }

  .entry {
    all: unset;
    cursor: default;
    border-radius: 3px;
  }

  .entry:hover,
  .entry:focus-visible {
    background: rgba(13, 23, 41, 0.08);
    outline: none;
  }

  .empty {
    font-style: italic;
    color: var(--ink-faint);
  }
</style>
