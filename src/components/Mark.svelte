<script lang="ts" module>
  /** Everything the board draws over a square, one shape per idea. The legend
      draws these same marks, so what it explains is what the board shows. */
  export type MarkKind =
    | 'zone'
    | 'line'
    | 'last'
    | 'highlight'
    | 'ring'
    | 'selected'
    | 'dot'
    | 'aim'
    | 'reticle'
    | 'target'
    | 'frame'
    | 'attacker'
    | 'queued'
    | 'ghost'
    | 'kill'
    | 'badge'
    | 'flash'
    | 'pending'
</script>

<script lang="ts">
  interface Props {
    kind: MarkKind
    /** The colour of a mark that belongs to a token: cyan for March, orange
        for Flank. */
    tone?: string
  }

  let { kind, tone }: Props = $props()
</script>

<span class={`mark ${kind}`} style={tone ? `--tone:${tone}` : undefined} aria-hidden="true">
  {#if kind === 'reticle' || kind === 'target' || kind === 'aim' || kind === 'badge'}
    <svg viewBox="0 0 40 40">
      <circle cx="20" cy="20" r="12" />
      <path d="M20 2v9M20 29v9M2 20h9M29 20h9" />
      {#if kind !== 'reticle'}<circle class="pip" cx="20" cy="20" r="3" />{/if}
    </svg>
  {:else if kind === 'kill'}
    <svg viewBox="0 0 40 40"><path d="M9 9l22 22M31 9 9 31" /></svg>
  {/if}
</span>

<style>
  .mark {
    position: absolute;
    inset: 0;
    pointer-events: none;
    --tone: var(--navy);
  }

  .mark svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  /* An Attack Zone, drawn as the sheet draws one: the square goes red. */
  .zone {
    background: rgba(227, 38, 43, 0.2);
  }

  /* The line a hovered slide would name, in that token's colour. */
  .line {
    background: color-mix(in srgb, var(--tone) 22%, transparent);
  }

  /* Where last turn's move went, and where it came from. */
  .last {
    background: rgba(185, 120, 42, 0.28);
  }

  .ghost {
    inset: 14%;
    border: 2px dashed var(--tone);
    border-radius: 50%;
    opacity: 0.85;
  }

  .highlight {
    box-shadow: inset 0 0 0 3px var(--navy);
    background: rgba(13, 23, 41, 0.1);
  }

  /* A piece this activation may move: a ring in the token's colour. */
  .ring {
    inset: 5%;
    border-radius: 50%;
    box-shadow: 0 0 0 2.5px var(--tone);
  }

  .selected {
    inset: 3%;
    border-radius: 50%;
    box-shadow:
      0 0 0 4px var(--tone),
      0 0 12px 3px color-mix(in srgb, var(--tone) 60%, transparent);
  }

  .dot::after {
    content: '';
    position: absolute;
    inset: 36%;
    border-radius: 50%;
    background: var(--tone);
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.7);
  }

  /* Taking aim: something your Attack can take, or would after this move. */
  .reticle,
  .target,
  .aim,
  .badge {
    color: var(--attack-deep);
  }

  .reticle svg,
  .target svg,
  .aim svg,
  .badge svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    stroke-linecap: round;
  }

  .reticle {
    inset: 8%;
    opacity: 0.85;
  }

  .target {
    inset: 4%;
    background: rgba(227, 38, 43, 0.16);
  }

  .target svg {
    stroke-width: 3.6;
    animation: aim 1.4s ease-in-out infinite;
  }

  .pip {
    fill: currentColor;
    stroke: none;
  }

  /* A destination that would bring a target into your Attack. */
  .aim {
    inset: 22%;
    color: var(--tone);
  }

  .aim svg {
    stroke: var(--attack-deep);
  }

  .aim .pip {
    fill: var(--tone);
  }

  /* On a piece: one of its moves would bring a target into your Attack. */
  .badge {
    inset: auto 2% 2% auto;
    width: 34%;
    height: 34%;
    background: var(--panel);
    border-radius: 50%;
    box-shadow: 0 0 0 1.5px var(--attack-deep);
  }

  .badge svg {
    inset: 12%;
    width: 76%;
    height: 76%;
  }

  /* Something the opponent's Attack can take next turn. */
  .frame {
    inset: 5%;
    border: 3px solid var(--attack);
    border-radius: 6px;
    background: rgba(227, 38, 43, 0.1);
  }

  .attacker {
    inset: 3%;
    border-radius: 50%;
    box-shadow:
      0 0 0 4px var(--attack),
      0 0 14px 4px rgba(227, 38, 43, 0.5);
  }

  .queued {
    inset: 6%;
    border-radius: 50%;
    border: 2.5px dashed var(--attack);
  }

  .kill {
    inset: 18%;
    color: var(--attack-deep);
  }

  .kill svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 5;
    stroke-linecap: round;
  }

  /* A tap on a touch screen has picked this; a second tap commits it. */
  .pending {
    inset: 2%;
    border: 3px solid var(--tone);
    border-radius: 6px;
    animation: aim 1s ease-in-out infinite;
  }

  .flash {
    animation: strike 0.8s ease-out forwards;
  }

  @keyframes strike {
    0% {
      background: rgba(227, 38, 43, 0.8);
    }
    100% {
      background: rgba(227, 38, 43, 0);
    }
  }

  @keyframes aim {
    50% {
      opacity: 0.55;
    }
  }
</style>
