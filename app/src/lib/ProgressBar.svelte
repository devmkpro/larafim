<script lang="ts">
  // Barra de progresso fina do Windows 11. `value` nulo = indeterminada.
  let { value = null }: { value?: number | null } = $props();
</script>

<div class="track" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={value === null ? undefined : Math.round(value * 100)}>
  {#if value === null}
    <div class="bar indeterminate"></div>
  {:else}
    <div class="bar" style:width="{Math.max(2, Math.min(100, value * 100))}%"></div>
  {/if}
</div>

<style>
  .track {
    position: relative;
    height: 3px;
    border-radius: 2px;
    background: var(--stroke-strong);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
    transition: width 0.15s linear;
  }
  .indeterminate {
    position: absolute;
    width: 40%;
    animation: slide 1.4s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      left: -40%;
    }
    to {
      left: 100%;
    }
  }
</style>
