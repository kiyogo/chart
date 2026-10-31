import { render } from 'solid-js/web';
import { App } from './App';
import './styles/main.css';

// Ensure browser favicon is always refreshed immediately without caching
const updateFavicon = () => {
  const iconSvg = `data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%232563EB' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z'/%3E%3C/svg%3E`;
  let link = document.querySelector("link[rel*='icon']") as HTMLLinkElement;
  if (!link) {
    link = document.createElement('link');
    link.rel = 'icon';
    link.type = 'image/svg+xml';
    document.head.appendChild(link);
  }
  link.href = iconSvg;
};
updateFavicon();

const root = document.getElementById('root');
if (root) {
  render(() => <App />, root);
}
