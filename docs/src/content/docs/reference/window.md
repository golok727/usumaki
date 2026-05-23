---
title: Window
description: Create and control native windows.
---

`Window` creates and controls a native OS window.

```tsx
import { Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';

const window = new Window('main', {
  width: 960,
  height: 680,
  title: 'My App',
  theme: 'system',
  rootStyles: { bg: '#09090b', color: '#f4f4f5', fontSize: 14 },
});

const root = createRoot(window);
root.render(<App />);
```

The first argument is a unique label. Use it to find an existing window later.

## Constructor

```ts
new Window(label: string, options?: WindowOptions);
```

## Options

| Option                                           | Description                                                |
| ------------------------------------------------ | ---------------------------------------------------------- |
| `width`, `height`                                | Initial content size.                                      |
| `title`                                          | Window title.                                              |
| `visible`                                        | Whether the window starts visible.                         |
| `resizable`                                      | Whether the window can be resized.                         |
| `decorations`                                    | Native titlebar and frame.                                 |
| `transparent`                                    | Transparent window background.                             |
| `maximized`, `minimized`, `fullscreen`           | Initial window state.                                      |
| `windowLevel`                                    | `normal`, `alwaysOnTop`, or `alwaysOnBottom`.              |
| `minWidth`, `minHeight`, `maxWidth`, `maxHeight` | Size constraints.                                          |
| `position`                                       | Initial `{ x, y }` screen position.                        |
| `theme`                                          | `light`, `dark`, or `system`.                              |
| `active`                                         | Whether the window should become active.                   |
| `contentProtected`                               | Ask the platform to prevent screen capture when supported. |
| `closable`, `minimizable`, `maximizable`         | Native window control availability.                        |
| `rootStyles`                                     | Props applied to the root element before render.           |

## Mutable Properties

```ts
window.title = 'Renamed';
window.visible = true;
window.resizable = false;
window.decorations = true;
window.transparent = false;
window.maximized = false;
window.minimized = false;
window.fullscreen = false;
window.windowLevel = 'alwaysOnTop';
window.theme = 'dark';
window.contentProtected = false;
window.closable = true;
window.minimizable = true;
window.maximizable = true;
```

## Methods

```ts
window.focus();
window.close();
window.requestRedraw();
window.setMinSize(720, 480);
window.setMaxSize(1600, 1200);
window.setPosition(120, 80);
```

## Getters

| Getter                      | Description                               |
| --------------------------- | ----------------------------------------- |
| `id`, `label`               | Internal id and the label you passed in.  |
| `innerWidth`, `innerHeight` | Current content size.                     |
| `innerSize`, `outerSize`    | Current size objects.                     |
| `position`                  | Current window position.                  |
| `scaleFactor`               | Platform scale factor.                    |
| `active`                    | Whether the window is active.             |
| `isDisposed`                | Whether the window has been disposed.     |
| `root`                      | Root element of the window's tree.        |
| `remBase`                   | Number of logical pixels used for `1rem`. |

`remBase` is mutable:

```ts
window.remBase = 18;
```

## Events

```ts
window.on('load', () => {});
window.on('resize', (event) => console.log(event.width, event.height));
window.on('close', () => {});
```

UI events bubble to the window, which is useful for global shortcuts.

## Multiple Windows

```ts
import { Window, getWindow } from 'uzumaki';

const existing = getWindow('main');
const main = existing ?? new Window('main', { width: 800, height: 600 });
```

Use stable labels such as `main`, `settings`, or `inspector`.
