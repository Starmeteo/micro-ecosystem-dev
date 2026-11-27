import { createRoot } from 'react-dom/client';
import { App } from './App';
import { native, viewId } from './world';
import './styles.css';

const root = document.querySelector<HTMLElement>('#root');
if (!root) throw new Error('Missing desktop root');
document.documentElement.dataset.view = viewId;
document.documentElement.dataset.native = String(native);
createRoot(root).render(<App />);
