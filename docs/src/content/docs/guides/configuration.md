---
title: Configure an App
description: Configure builds, packaging, resources, and app identity.
---

`uzumaki.config.json` describes how the CLI should build and package your app.

Most projects need three things:

- App identity: name, version, and identifier.
- Optional build command: how to turn TypeScript into JavaScript.
- Bundle settings: resources, where the built JavaScript lives, and what executable to create.

## Example Config

```json
{
  "productName": "my-app",
  "version": "0.1.0",
  "identifier": "com.example.my_app",
  "jsxImportSource": "uzumaki-react",
  "beforeBuildCommand": "bun build src/index.tsx --target node --outdir dist --minify --external uzumaki",
  "bundle": {
    "resources": ["assets/**/*"],
    "js": {
      "rootDir": "./dist",
      "entry": "index.js"
    },
    "outputDir": "./target",
    "binName": "my-app"
  }
}
```

## App Identity

| Field         | Description                                       |
| ------------- | ------------------------------------------------- |
| `productName` | Human-readable app name.                          |
| `version`     | App version.                                      |
| `identifier`  | Reverse-DNS app id, such as `com.example.my_app`. |

Use a stable identifier early. It is also exposed at runtime as `Uz.path.identifier`.

## Build Settings

```json
{
  "beforeBuildCommand": "bun build src/index.tsx --target node --outdir dist --minify --external uzumaki"
}
```

`beforeBuildCommand` is optional. When present, it runs before packaging and should write JavaScript into `bundle.js.rootDir`.

Always mark `uzumaki` as external:

```sh
bun build src/index.tsx --target node --outdir dist --minify --external uzumaki
```

Uzumaki provides the `uzumaki` module when your app runs. Bundling it into your app would produce the wrong module.

Any bundler that can externalize a module works — `bun build`, esbuild, Rollup, tsup, and so on. The scaffolded command uses `bun build` because it is fast and ships with the toolchain.

## Bundle Settings

| Field               | Description                                        |
| ------------------- | -------------------------------------------------- |
| `bundle.resources`  | Resource globs copied into the packaged app.       |
| `bundle.js.rootDir` | Directory containing built JavaScript.             |
| `bundle.js.entry`   | Entry file inside `bundle.js.rootDir`.             |
| `bundle.outputDir`  | Directory where packaged output is written.        |
| `bundle.binName`    | Executable name. Defaults to `productName`.        |
| `bundle.baseBinary` | Optional host binary to use as the package source. |

## Resources

Declare resources:

```json
{
  "bundle": {
    "resources": ["assets/**/*", "data/**/*.json"]
  }
}
```

Resolve them:

```tsx
const icon = Uz.path.resource('assets/icons/search.svg');
const config = Uz.path.resource('data/app.json');
```

`<image src>` accepts bundled paths, absolute paths, `file://` URLs, and `https://` URLs.

## CLI Flags

```sh
uzumaki build --config path/to/uzumaki.config.json
uzumaki build --no-build
```

Use `--no-build` in CI when your JavaScript bundle has already been produced.
