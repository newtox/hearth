<script lang="ts">
  const PATHS: Record<string, string> = {
    plus: 'M12 5v14M5 12h14',
    close: 'M6 6l12 12M18 6L6 18',
    gear: 'M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM19.4 13a7.6 7.6 0 0 0 0-2l2-1.5-2-3.5-2.4 1a7.6 7.6 0 0 0-1.7-1L15 3.5h-4l-.3 2.5a7.6 7.6 0 0 0-1.7 1l-2.4-1-2 3.5 2 1.5a7.6 7.6 0 0 0 0 2l-2 1.5 2 3.5 2.4-1a7.6 7.6 0 0 0 1.7 1l.3 2.5h4l.3-2.5a7.6 7.6 0 0 0 1.7-1l2.4 1 2-3.5z',
    hash: 'M9 4L7 20M17 4l-2 16M4 9h17M3 15h17',
    users: 'M16 19v-1a4 4 0 0 0-4-4H7a4 4 0 0 0-4 4v1M9.5 10a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM21 19v-1a4 4 0 0 0-3-3.9M15.5 3.2a3.5 3.5 0 0 1 0 6.6',
    menu: 'M4 7h16M4 12h16M4 17h16',
    send: 'M4 12l16-8-6 16-3-7z',
    edit: 'M4 20h4L19 9l-4-4L4 16zM14 6l4 4',
    trash: 'M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13',
    crown: 'M4 18h16M4 18L3 8l5 4 4-7 4 7 5-4-1 10',
    mail: 'M3 6h18v12H3zM3 7l9 7 9-7',
    up: 'M6 15l6-6 6 6',
    down: 'M6 9l6 6 6-6',
    mic: 'M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zM5 11a7 7 0 0 0 14 0M12 18v3',
    micOff: 'M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zM5 11a7 7 0 0 0 14 0M12 18v3M4 4l16 16',
    screen: 'M3 5h18v11H3zM8 20h8M12 16v4',
    volume: 'M4 9v6h4l5 4V5L8 9zM16 9a4 4 0 0 1 0 6M18.5 6.5a8 8 0 0 1 0 11',
    smile: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM8.5 14.5a4.5 4.5 0 0 0 7 0M9 10h.01M15 10h.01',
    clip: 'M20 11.5l-8.5 8.5a5 5 0 0 1-7-7L13 4.5a3.3 3.3 0 0 1 4.7 4.7L9.5 17.4a1.7 1.7 0 0 1-2.4-2.4L14.5 7.6',
    leave: 'M15 4h4v16h-4M10 8l-4 4 4 4M6 12h10',
    gamepad: 'M6 8h12a4 4 0 0 1 4 4v2a3 3 0 0 1-5.5 1.7L15.5 14h-7l-1 1.7A3 3 0 0 1 2 14v-2a4 4 0 0 1 4-4zM7 10v3M5.5 11.5h3M16 11h.01M18 12.5h.01',
    broadcast: 'M12 13a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3zM8.5 15a5 5 0 0 1 0-7M15.5 8a5 5 0 0 1 0 7M5.5 18a9.5 9.5 0 0 1 0-13M18.5 5a9.5 9.5 0 0 1 0 13',
    lock: 'M6 11h12v9H6zM8.5 11V8a3.5 3.5 0 0 1 7 0v3',
    phone: 'M5 4h4l2 5-2.5 1.5a11 11 0 0 0 5 5L15 13l5 2v4a2 2 0 0 1-2 2A16 16 0 0 1 3 6a2 2 0 0 1 2-2z',
    userPlus: 'M15 19v-1a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v1M8.5 10a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM19 8v6M16 11h6',
    copy: 'M9 9h11v11H9zM5 15H4V4h11v1',
  };
  let { name, size = 18 }: { name: string; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>
