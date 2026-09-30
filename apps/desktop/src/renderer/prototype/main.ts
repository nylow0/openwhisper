// PROTOTYPE entry: `bun run prototype` opens /prototype.html?variant=A
import Prototype from './Prototype.svelte';
import '../app.css';
import 'flag-icons/css/flag-icons.min.css';

const el = document.getElementById('app');
if (!el) throw new Error('Missing #app element');

export default new Prototype({ target: el });
