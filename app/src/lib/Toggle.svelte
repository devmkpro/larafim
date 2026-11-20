<script lang="ts">
  // Interruptor no estilo do Windows 11.
  let {
    checked = false,
    disabled = false,
    label,
    onchange,
  }: { checked?: boolean; disabled?: boolean; label: string; onchange?: (value: boolean) => void } = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange?.(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 40px;
    height: 20px;
    padding: 0;
    border-radius: 10px;
    border: 1px solid var(--text-2);
    background: transparent;
    flex: none;
  }
  .toggle:hover:not(:disabled) {
    background: var(--subtle-hover);
  }
  .knob {
    position: absolute;
    top: 50%;
    left: 4px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-2);
    transform: translateY(-50%);
    transition: left 0.15s ease, width 0.1s;
  }
  .toggle:hover:not(:disabled) .knob {
    width: 14px;
    height: 14px;
  }
  .toggle.on,
  .toggle.on:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
  }
  .toggle.on:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .toggle.on .knob {
    left: 22px;
    background: var(--accent-fg);
  }
  .toggle.on:hover:not(:disabled) .knob {
    left: 21px;
  }
  .toggle:disabled {
    border-color: var(--text-disabled);
  }
  .toggle:disabled .knob {
    background: var(--text-disabled);
  }
</style>
