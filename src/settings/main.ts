import { mount } from 'svelte';
import SettingsApp from './SettingsApp.svelte';

const target = document.getElementById('app');
if (target) {
  mount(SettingsApp, { target });
}
