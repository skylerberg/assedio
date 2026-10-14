<script lang="ts">
  interface Props {
    /** Which way the arrowed rule points: toward the title it flanks. */
    side: 'left' | 'right'
  }

  let { side }: Props = $props()
</script>

<!-- The rules sheet flanks its title with a bronze knot and an arrowed rule;
     the knot is the Flank token's motif. -->
<span class="ornament" class:right={side === 'right'} aria-hidden="true">
  <svg class="rule" viewBox="0 0 120 10" preserveAspectRatio="none">
    <path d="M0 5.4 112 4.4v1.2Z" fill="var(--copper)" />
    <path d="M111 1.6 119 5l-8 3.4 2-3.4Z" fill="var(--copper)" />
  </svg>
  <svg class="knot" viewBox="0 0 40 40">
    <g stroke="var(--copper-deep)" stroke-width="0.9" stroke-linejoin="round">
      <g fill="var(--copper)">
        <path d="M11 11 5.6 5.6M29 11l5.4-5.4M11 29l-5.4 5.4M29 29l5.4 5.4" stroke-width="2.4" />
        <path d="M2.5 2.5 9.5 4 4 9.5Z" />
        <path d="M37.5 2.5 30.5 4 36 9.5Z" />
        <path d="M2.5 37.5 9.5 36 4 30.5Z" />
        <path d="M37.5 37.5 30.5 36 36 30.5Z" />
        <rect x="10" y="10" width="20" height="20" />
        <path d="M20 6.5 22.2 10h-4.4ZM20 33.5l2.2-3.5h-4.4ZM6.5 20l3.5-2.2v4.4ZM33.5 20 30 17.8v4.4Z" />
      </g>
      <rect x="13.5" y="13.5" width="13" height="13" fill="var(--peach)" />
      <rect x="17" y="17" width="6" height="6" fill="var(--copper)" />
    </g>
  </svg>
</span>

<style>
  .ornament {
    display: inline-flex;
    align-items: center;
    gap: 0.3em;
    flex: 0 1 auto;
    min-width: 0;
  }

  .ornament.right {
    flex-direction: row-reverse;
  }

  .ornament.right .rule {
    transform: scaleX(-1);
  }

  .rule {
    width: var(--rule, 3.2em);
    min-width: 0;
    flex: 0 1 auto;
    height: 0.5em;
  }

  .knot {
    width: 1.15em;
    height: 1.15em;
    flex: none;
  }
</style>
