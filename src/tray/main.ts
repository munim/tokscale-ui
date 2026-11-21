import { mount } from 'svelte';
import '../styles/tokens.css';
import TrayApp from './TrayApp.svelte';

const app = mount(TrayApp, {
  target: document.getElementById('app')!,
});

export default app;
