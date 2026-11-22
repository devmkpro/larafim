<script lang="ts">
  // Marca do Larafim (versão simples de app/app-icon.svg, sem textura nem
  // sombra, para ficar nítida em tamanhos pequenos).
  let { size = 16 }: { size?: number } = $props();
</script>

<svg class="logo" width={size} height={size} viewBox="-3 0 307 307" aria-hidden="true">
  <rect x="0" y="0" width="300" height="307" rx="70" fill="#0F7B74" />
  <rect x="135" y="66" width="100" height="46" rx="23" fill="#FFFFFF" />
  <rect x="60" y="130" width="180" height="46" rx="23" fill="#BFEEE9" />
  <rect x="60" y="194" width="90" height="46" rx="23" fill="#5FC7BD" />
  <circle cx="160" cy="89.2" r="10.2" fill="#0F7B74" />
</svg>

<style>
  .logo {
    flex: none;
    display: block;
  }
</style>
