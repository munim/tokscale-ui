import { mount } from 'svelte';
import '../styles/tokens.css';
import DashboardApp from './DashboardApp.svelte';

const app = mount(DashboardApp, {
  target: document.getElementById('app')!,
});

export default app;
