import './lib/tooltip';
import './lib/verify';
import './lib/external';
import '@fontsource/poppins/400.css';
import '@fontsource/poppins/500.css';
import '@fontsource/poppins/600.css';
import '@fontsource/poppins/700.css';
import '@fontsource-variable/alegreya';
import '@fontsource/almendra/400.css';
import '@fontsource/almendra/700.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

mount(App, { target: document.getElementById('app')! });
