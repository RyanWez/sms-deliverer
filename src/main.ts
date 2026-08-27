import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

if ((import.meta as any).env?.DEV) {
  import('$lib/utils/synthetic');
}

const app = mount(App, {
  target: document.getElementById('app')!,
});

export default app;
