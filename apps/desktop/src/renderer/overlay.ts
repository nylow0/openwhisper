if (import.meta.env.DEV && new URLSearchParams(location.search).has('demo')) {
  await import('./demo-mock');
}

import Overlay from './Overlay.svelte';
import './overlay.css';

const el = document.getElementById('overlay');
if (!el) throw new Error('Missing #overlay element');

const overlay = new Overlay({
  target: el,
});

export default overlay;
