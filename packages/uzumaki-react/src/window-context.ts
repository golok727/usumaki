import { createContext, createElement, useContext } from 'react';
import { Window } from 'uzumaki';

const WindowContext = createContext<Window | null>(null);

export function WindowProvider({
  children,
  window,
}: {
  children: React.ReactNode;
  window: Window;
}) {
  return createElement(WindowContext, { value: window }, children);
}

/**
 * Returns the window this component is rendering in.
 */
export function useWindow() {
  const window = useContext(WindowContext);
  if (!window) throw new Error('WindowContext is not available');
  return window;
}
