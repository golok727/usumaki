<div align="center">
  <img src="etc/logo.svg" width="140" alt="Uzumak" />

  <h1>Uzumaki</h1>

  <p>Native desktop UI framework for JavaScript and TypeScript</p>

[![CI](https://github.com/golok727/uzumaki/actions/workflows/ci.yml/badge.svg)](https://github.com/golok727/uzumaki/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![GitHub stars](https://img.shields.io/github/stars/golok727/uzumaki?style=flat&logo=github)](https://github.com/golok727/uzumaki/stargazers)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![Powered by Deno](https://img.shields.io/badge/runtime-Deno-black?logo=deno)](https://deno.com/)
[![wgpu](https://img.shields.io/badge/renderer-wgpu%20%2B%20vello-purple)](#)

</div>

> [!NOTE]
> Uzumaki is in early alpha. Expect breaking changes between releases.

## Quick Example

```tsx
import { useState } from 'react';
import { Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'My App',
});

function App() {
  const [count, setCount] = useState(0);

  return (
    <view
      display="flex"
      flexDir="col"
      w="full"
      h="full"
      items="center"
      justify="center"
      bg="#0f0f0f"
      gap={16}
    >
      <text fontSize={32} fontWeight={700} color="#e4e4e7">
        Welcome to Uzumaki
      </text>
      <text fontSize={18} color="#a1a1aa">
        Count: {count}
      </text>
      <button
        onClick={() => setCount((value) => value + 1)}
        px={24}
        py={10}
        rounded={8}
        bg="#2d2d30"
        hover:bg="#3e3e42"
      >
        <text fontSize={16} color="#60a5fa">
          Increment
        </text>
      </button>
    </view>
  );
}

createRoot(window).render(<App />);
```

## Package Model

- `uzumaki` is a built-in runtime module
- `uzumaki-react` provides the React renderer
- `uzumaki-types` provides TypeScript declarations for the built-in runtime

## Install

**macOS / Linux**

```sh
curl -fsSL https://uzumaki.run/install.sh | sh
```

**Windows**

```powershell
irm https://uzumaki.run/install.ps1 | iex
```

Then create a project:

```sh
uzumaki init my-app
cd my-app
pnpm install
pnpm dev
```

## Links

- [Docs](https://uzumaki.run)
- [GitHub](https://github.com/golok727/uzumaki)
- [Contributing](CONTRIBUTING.md)
- [Development](DEVELOPMENT.md)

## Acknowledgements

Uzumaki builds on great work from:

- [Deno](https://github.com/denoland/deno) for the JS/TS runtime (`deno_core`, `deno_runtime`)
- [wgpu](https://github.com/gfx-rs/wgpu) for the GPU backend
- [Vello](https://github.com/linebender/vello) for rendering
- [Parley](https://github.com/linebender/parley) for text layout
- [Taffy](https://github.com/DioxusLabs/taffy) for layout
- [Blitz](https://github.com/DioxusLabs/blitz), where i learned how to drive the Linebender stack tysmm <333 and lovingly stole some ideas
- [Zed](https://github.com/zed-industries/zed), i borrowed a few utilities and they inspired me to start making ui frameworks a few years ago <3

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
