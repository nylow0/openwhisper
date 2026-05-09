import App from './App.svelte';
import './app.css';

const el = document.getElementById('app');
if (!el) throw new Error('Missing #app element');

const app = new App({
  target: el
});

export default app;