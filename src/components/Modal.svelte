<script lang="ts">
  import { onMount } from 'svelte'
  import type { Snippet } from 'svelte'

  interface Props {
    label: string
    onclose: () => void
    /** A narrow card rather than a reading sheet. */
    compact?: boolean
    children: Snippet
  }

  let { label, onclose, compact = false, children }: Props = $props()
  let dialog: HTMLDivElement | null = $state(null)

  // Focus goes in when the dialog opens and back where it was when it closes,
  // so a keyboard player is never left on a control behind the scrim.
  onMount(() => {
    const before = document.activeElement as HTMLElement | null
    const first = dialog?.querySelector<HTMLElement>('[data-autofocus]') ?? dialog
    first?.focus()
    return () => before?.focus?.()
  })

  /** Tab cycles within the dialog rather than escaping to the page beneath. */
  function trap(event: KeyboardEvent) {
    if (event.key !== 'Tab' || !dialog) return
    const focusable = [
      ...dialog.querySelectorAll<HTMLElement>('button:not(:disabled), [href], input, [tabindex="0"]'),
    ]
    if (focusable.length === 0) return
    const first = focusable[0]
    const last = focusable[focusable.length - 1]
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    }
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      event.stopPropagation()
      onclose()
    }
  }}
/>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" onclick={onclose}>
  <div
    class="panel sheet"
    class:compact
    role="dialog"
    aria-modal="true"
    aria-label={label}
    tabindex="-1"
    bind:this={dialog}
    onclick={(event) => event.stopPropagation()}
    onkeydown={trap}
  >
    {@render children()}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(13, 23, 41, 0.5);
    display: grid;
    place-items: center;
    padding: 1rem;
    animation: fade-in 0.15s ease-out;
  }

  .sheet {
    width: min(48rem, 100%);
    max-height: min(92dvh, 60rem);
    overflow-y: auto;
    text-align: left;
    animation: rise 0.2s ease-out;
  }

  .sheet:focus {
    outline: none;
  }

  .sheet.compact {
    width: min(26rem, 100%);
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  @keyframes rise {
    from {
      transform: translateY(10px);
      opacity: 0;
    }
  }
</style>
