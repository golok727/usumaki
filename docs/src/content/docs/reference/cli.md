---
title: CLI
description: Run scripts, scaffold projects, build apps, and upgrade the Uzumaki CLI.
---

The `uzumaki` CLI is a self-contained desktop UI host for JavaScript and TypeScript. It is built on Deno, so you do not need Node.js, Deno, or Bun installed to run a script.

```sh
uzumaki ./app.tsx
```

This starts the application loop and gives you access to the Uzumaki APIs

## Run Modes

By default, `uzumaki <entry>` runs your script in **GUI mode** — it boots Uzumaki, opens windows, and dispatches events. Use `run` for headless scripts that just want the JavaScript engine without any UI.

```sh
uzumaki ./app.tsx              # GUI mode (alias for `dev`)
uzumaki dev ./app.tsx          # GUI mode, explicit
uzumaki run ./script.ts        # headless mode, no window
```

Anything after the entry file is forwarded to your script as arguments:

```sh
uzumaki ./app.tsx --port 3000
```

## Commands

| Command           | Use it for                                                 |
| ----------------- | ---------------------------------------------------------- |
| `uzumaki <entry>` | Run a TypeScript or JavaScript file in GUI mode.           |
| `uzumaki dev`     | Same as the bare form — opens an entry in a native window. |
| `uzumaki run`     | Run an entry in headless mode (no window).                 |
| `uzumaki create`  | Create a new project in a new directory.                   |
| `uzumaki init`    | Initialize a project in the current directory.             |
| `uzumaki build`   | Build and package an app using `uzumaki.config.json`.      |
| `uzumaki upgrade` | Upgrade to the latest CLI version.                         |

Run `uzumaki <command> --help` for detailed flags.

## Create a Project

```sh
uzumaki create my-app          # scaffolds my-app/ in the current directory
uzumaki create                 # prompts for a name
uzumaki init                   # scaffold into the current directory
```

The scaffold wires up `uzumaki.config.json`, a TypeScript entry file, and a minimal React app. Install dependencies with the package manager of your choice:

```sh
cd my-app
pnpm install
pnpm dev
```

## Build for Distribution

`uzumaki build` reads `uzumaki.config.json` from the current directory (or any ancestor) and produces a standalone executable.

```sh
uzumaki build
uzumaki build --config ./custom.config.json
uzumaki build --no-build         # skip the build step, only package
```

The config tells the CLI which optional command to run before packaging, where the bundled JS lives, and how to name the output. See the [build guide](/guides/building/) for the full schema.

## Upgrade

```sh
uzumaki upgrade                  # install the latest release
uzumaki upgrade --version 0.2.0  # pin to a specific version
```

## Version

```sh
uzumaki --version                # short version
uzumaki -V                       # short version
```

The long form prints the V8 and TypeScript versions bundled with this build:

```sh
uzumaki --help
```
