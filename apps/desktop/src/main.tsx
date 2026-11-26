import { createRoot } from 'react-dom/client';
import { App } from './App';
import { native, viewId } from './world';
import { r5Preview } from './r5Preview';
import './styles.css';

const root = document.querySelector<HTMLElement>('#root');
if (!root) throw new Error('Missing desktop root');
document.documentElement.dataset.view = viewId;
document.documentElement.dataset.native = String(native);
if (r5Preview) document.documentElement.dataset.r5Preview = 'true';
createRoot(root).render(<App />);
