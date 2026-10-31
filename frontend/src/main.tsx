import { render } from 'solid-js/web';
import { App } from './App';
import './styles/main.css';

// Ensure browser favicon is always refreshed immediately without caching
const updateFavicon = () => {
  const iconSvg = `data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='%232563EB'%3E%3Cpath d='M20 2H4C2.9 2 2 2.9 2 4v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z'/%3E%3C/svg%3E`;
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
