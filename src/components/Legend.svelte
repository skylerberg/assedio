<script lang="ts" module>
  import type { MarkKind } from './Mark.svelte'

  export type LegendKey =
    | 'slide'
    | 'movable'
    | 'destination'
    | 'aim'
    | 'reticle'
    | 'target'
    | 'frame'
    | 'zone'
    | 'attacker'
    | 'last'
    | 'kill'

  /** One key the board is using right now, in the colour it is using it. */
  export interface LegendEntry {
    key: LegendKey
    tone?: string
  }

  export const LEGEND_MARKS: Record<LegendKey, MarkKind> = {
    slide: 'ghost',
    movable: 'ring',
    destination: 'dot',
    aim: 'aim',
    reticle: 'reticle',
    target: 'target',
    frame: 'frame',
    zone: 'zone',
    attacker: 'attacker',
    last: 'last',
    kill: 'kill',
  }
</script>

<script lang="ts">
  import Mark from './Mark.svelte'

  interface Props {
    entries: LegendEntry[]
    /** Whose Attacks the reticle and the frame belong to: "your" and "Umber's"
        in a game against the machine, the sides' names when both are human. */
    mine: string
    theirs: string
  }

  let { entries, mine, theirs }: Props = $props()

  function label(key: LegendKey): string {
    switch (key) {
      case 'slide':
        return 'slide here'
      case 'movable':
        return 'can move'
      case 'destination':
        return 'can go here'
      case 'aim':
        return 'brings a target into reach'
      case 'reticle':
        return `${mine} Attack can take`
      case 'target':
        return 'take it now'
      case 'frame':
        return `${theirs} Attack can take`
      case 'zone':
        return 'Attack Zone'
      case 'attacker':
        return 'attacking now'
      case 'last':
        return 'last turn'
      case 'kill':
        return 'killed'
    }
  }
</script>

<ul class="legend" aria-label="Key to the board">
  {#each entries as entry (entry.key)}
    <li>
      <span class="swatch"><Mark kind={LEGEND_MARKS[entry.key]} tone={entry.tone} /></span>
      {label(entry.key)}
    </li>
  {/each}
</ul>

<style>
  /* Two rows of swatches are always kept, as the hint keeps two lines: hovering
     the board adds keys, and a key that wrapped onto a new row would grow the
     panel and shrink the board above it. */
  .legend {
    --row: 1.15rem;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-content: flex-start;
    min-height: calc(2 * var(--row) + 0.25rem);
    gap: 0.25rem 0.9rem;
    line-height: var(--row);
    list-style: none;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--ink-soft);
  }

  li {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .swatch {
    position: relative;
    width: var(--row);
    height: var(--row);
    flex: none;
    background: var(--cell);
    box-shadow: inset 0 0 0 1px var(--board-line);
  }
</style>
