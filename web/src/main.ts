import './lib/tooltip';
import './lib/verify';
import './lib/external';
import './lib/validation';
import '@fontsource-variable/figtree';
import '@fontsource-variable/bricolage-grotesque';
import '@fontsource-variable/alegreya';
import '@fontsource/almendra/400.css';
import '@fontsource/almendra/700.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

mount(App, { target: document.getElementById('app')! });
