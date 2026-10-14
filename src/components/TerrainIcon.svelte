<script lang="ts">
  interface Props {
    kind: 'castle' | 'hilltop'
    /** Omit to fill the parent, which is how the board scales terrain with
        the cell. */
    size?: number
  }

  let { kind, size }: Props = $props()
</script>

<!-- Drawn after the rules sheet: a sepia castle of three towers round a keep,
     and a hill of brush strokes. -->
{#if kind === 'castle'}
  <svg
    class="terrain-icon"
    style={size === undefined ? undefined : `width:${size}px;height:${size}px`}
    viewBox="0 0 64 60"
    aria-hidden="true"
  >
    <g
      fill="var(--castle-fill)"
      stroke="var(--castle-ink)"
      stroke-width="1.6"
      stroke-linejoin="round"
      stroke-linecap="round"
    >
      <path d="M16 56V30h2.2v-3h2.4v3h2.2v-3h2.4v3H27v26Z" />
      <path d="M37 56V30h2.2v-3h2.4v3h2.2v-3h2.4v3H48v26Z" />
      <path d="M6 56V22H5v-6h2.6v2.6h2.2V16h2.6v2.6h2.2V16H17v6h-1v34Z" />
      <path d="M48 56V22h-1v-6h2.6v2.6h2.2V16h2.6v2.6h2.2V16H59v6h-1v34Z" />
      <path
        d="M24 56V16h-1.5v-6h2.8v2.6h2.4V10h2.6v2.6h2.4V10h2.6v2.6h2.4V10h2.8v6H40v40Z"
      />
      <path d="M32 2.6l6.4 2-6.4 2.3Z" />
    </g>
    <g fill="none" stroke="var(--castle-ink)" stroke-width="1.4" stroke-linecap="round">
      <path d="M32 10V1.8" />
      <path d="M28.4 56v-7.4a3.6 3.6 0 0 1 7.2 0V56" />
      <path d="M27 24.2v-2.4a1.1 1.1 0 0 1 2.2 0v2.4Z" />
      <path d="M30.9 24.2v-2.4a1.1 1.1 0 0 1 2.2 0v2.4Z" />
      <path d="M34.8 24.2v-2.4a1.1 1.1 0 0 1 2.2 0v2.4Z" />
      <path d="M18.6 38v-2a1 1 0 0 1 2 0v2Z" />
      <path d="M22.4 38v-2a1 1 0 0 1 2 0v2Z" />
      <path d="M39.6 38v-2a1 1 0 0 1 2 0v2Z" />
      <path d="M43.4 38v-2a1 1 0 0 1 2 0v2Z" />
      <path d="M11 27.5v4.5M53 27.5v4.5M11 40v4M53 40v4" />
      <path d="M2.5 56.4h59" />
    </g>
  </svg>
{:else}
  <svg
    class="terrain-icon"
    style={size === undefined ? undefined : `width:${size}px;height:${size}px`}
    viewBox="0 0 64 46"
    aria-hidden="true"
  >
    <g fill="var(--hill-ink)">
      <path d="M5 34C14 26 22 8 33 8s17 16 27 25c-10-7-16-19.5-27-19.5S15 27 5 34Z" />
      <path d="M14 30c6-6 12-14 20-14s13 6 19 13c-6-5-11-10.5-19-10.5S21 25 14 30Z" />
      <path d="M20 34c6-6 10-11 17-11s12 5 19 10c-7-3-12-7.5-19-7.5S26 30 20 34Z" />
      <path d="M9 36c9-3 21-6 33-5 8 .5 14 3 19 6-6-1.5-12-3.4-19-3.4-11-.4-23 1.9-33 2.4Z" />
      <path d="M4 40.2c10-1 18-.6 26-.8-4 1-16 1.2-26 .8Z" />
      <path d="M34 41.2c8-1 18-.8 26-.4-8 .8-18 1-26 .4Z" />
      <path d="M16 43.6c8-.6 18-.4 28 0-10 .6-20 .6-28 0Z" />
    </g>
  </svg>
{/if}

<style>
  .terrain-icon {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
