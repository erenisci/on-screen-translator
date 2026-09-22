import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import type { ReactElement } from 'react';
import '../styles/index.css';

/** Shared bootstrap for all three windows. */
export function mount(element: ReactElement): void {
  const container = document.getElementById('root');
  if (!container) throw new Error('#root is missing from the window HTML');
  createRoot(container).render(<StrictMode>{element}</StrictMode>);
}
