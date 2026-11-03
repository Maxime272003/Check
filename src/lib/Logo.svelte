<script>
  let { size = 22 } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
  <rect x="8" y="8" width="52" height="52" rx="16" fill="#1A1033" />
  <rect x="4" y="4" width="52" height="52" rx="16" fill="#FF3D8B" stroke="#1A1033" stroke-width="3" />
  <path d="M17 31l8 8 17-19" fill="none" stroke="#1A1033" stroke-width="11" stroke-linecap="round" stroke-linejoin="round" />
  <path d="M17 31l8 8 17-19" fill="none" stroke="#FFE14D" stroke-width="5.5" stroke-linecap="round" stroke-linejoin="round" />
</svg>
