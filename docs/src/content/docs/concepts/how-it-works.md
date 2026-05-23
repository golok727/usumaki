---
title: How Uzumaki Works
description: The mental model for windows, elements, events, and React in Uzumaki.
---

Uzumaki is a program that runs your TypeScript app, opens a real OS window, and draws the UI inside it. Think of it the way you think of a browser: a host that loads your code and turns it into UI. The difference is that the contents are not a web page and not OS widgets either — Uzumaki paints them itself, the way Flutter or a game engine does. The window is native; what's inside is whatever you describe in JSX.

You only deal with three things:

1. **`Window`** — an OS window you create from JavaScript.
2. **JSX elements** like `<view>`, `<text>`, `<button>` — the things Uzumaki knows how to draw.
3. **The `uzumaki` module** — the built-in API for window control, clipboard, paths, and so on.

Everything else (React, the renderer, layout) is built on top of those three.

## A Window Is a Real Window

```tsx
import { Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';

const window = new Window('main', { width: 900, height: 620 });
const root = createRoot(window);
root.render(<App />);
```

`Window` is not the browser `window`. It is an OS-level window you create, move, resize, and listen to. The first argument is a label so you can look it up later with `getWindow('main')`.

## JSX Elements Are Uzumaki's Own, Not HTML

Uzumaki JSX looks like React, but the tags are not DOM tags:

```tsx
<view display="flex" flexDir="col" gap={12}>
  Hello
  <button onClick={save}>Save</button>
</view>
```

There is no `<div>`, no `<span>`, no `<img>`. Each Uzumaki element is a primitive Uzumaki knows how to draw — a layout box, a pressable, an input, an inline text run. The full set lives in [Elements](/reference/elements/).

Plain strings render as text inside any element, and typography props (`fontSize`, `fontWeight`, `color`) work on any element too. `<text>` is the one **inline** element — every other element is block-level. Reach for it when you want a styled run to flow inline:

```tsx
<view fontSize={16}>
  Welcome back, <text fontWeight={700}>Ada</text>.
</view>
```

If you have used React Native, this is the same idea: React is the programming model, but the building blocks are not HTML.

## Styling Is Props, Not CSS

There is no CSS, no class names, no stylesheet. Styles are attributes on the elements themselves:

```tsx
<view p={20} rounded={16} bg="#18181b" border={1} borderColor="#27272a" />
```

Numbers are logical pixels. Strings cover special values like `"full"`, `"50%"`, `"auto"`, and `"1.25rem"`. State variants use prefixes:

```tsx
<button bg="#27272a" hover:bg="#3f3f46" active:scale={0.98} />
```

See [Props](/reference/props/) for the full list.

## Events Bubble Through the Element Tree

Events flow capture → target → bubble, the same shape as the DOM, but they travel through Uzumaki's element tree instead. Handlers receive Uzumaki event objects with `target`, `currentTarget`, `preventDefault()`, and the usual propagation controls:

```tsx
<button
  onClick={(event) => {
    event.preventDefault();
    save();
  }}
>
  <text>Save</text>
</button>
```

Window-level events (open, resize, close, global keys) live on the `Window` instance:

```ts
window.on('resize', (event) => {
  console.log(event.width, event.height);
});
```

## React Drives the Tree

`uzumaki-react` is a thin adapter that lets React manage Uzumaki elements. You write components, hooks, and state the way you already do. When your React tree changes, `uzumaki-react` applies the diff to the live element tree and Uzumaki repaints.

```tsx
function Counter() {
  const [count, setCount] = useState(0);
  return (
    <button onClick={() => setCount(count + 1)}>
      <text>Clicked {count} times</text>
    </button>
  );
}
```

React is the first adapter Uzumaki ships. Solid, Vue, and Svelte adapters are on the roadmap. The rest of the docs assume React.

## The `uzumaki` Module Is Built In

`Window`, `Clipboard`, `Uz.path`, event classes, and the element classes all live in the built-in `uzumaki` module:

```ts
import { Window, Clipboard, RUNTIME_VERSION } from 'uzumaki';
```

You import it like any package, but you do not install it from npm. Uzumaki provides it when your app starts. When you bundle your app, mark `uzumaki` as **external** so the bundler does not try to inline it.

## The Loop, End to End

Putting it together:

1. Your entry file creates a `Window`, then `const root = createRoot(window); root.render(<App />)`.
2. `uzumaki-react` walks your JSX and creates the matching Uzumaki elements under the window's root.
3. Uzumaki lays them out and paints them.
4. The user clicks. Uzumaki dispatches an event up the element tree to your handler.
5. Your handler updates React state. React re-renders.
6. `uzumaki-react` applies the diff to the live elements. Uzumaki repaints.

That is the whole loop. Everything else is filling in shapes, styles, and behaviors.

## Where to Go Next

- [React in Uzumaki](/concepts/react-runtime/) — JSX setup, refs, and what React does (and does not) do here.
- [Quick Start](/guides/quick-start/) — build a small app from scratch.
- [Elements](/reference/elements/) and [Props](/reference/props/) — the catalog you reach for while building.
- [Runtime API](/reference/runtime-api/) — when you want imperative control instead of React.
