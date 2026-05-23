---
title: Package for Distribution
description: Build JavaScript, copy resources, and package your app.
---

`uzumaki build` packages the app described by `uzumaki.config.json`.

```sh
uzumaki build
```

When `beforeBuildCommand` is present, the command runs it before packaging. It copies declared resources and writes packaged output to `bundle.outputDir`.

## Local Build

```sh
pnpm install
uzumaki build
```

Use this when you want the CLI to run your configured JavaScript build before packaging. Omit `beforeBuildCommand` when another tool already produces the JavaScript bundle.

## CI Build

Bundle JavaScript yourself, then package without rerunning the build step:

```sh
pnpm install
pnpm build
uzumaki build --no-build
```

This keeps CI explicit and avoids running the same bundler twice.

## Resources in Packaged Apps

Files matched by `bundle.resources` are copied into the packaged app:

```json
{
  "bundle": {
    "resources": ["assets/**/*"]
  }
}
```

Resolve them with `Uz.path.resource(...)`:

```tsx
const logo = Uz.path.resource('assets/logo.svg');
```

The same call should work in development and packaged builds.

## Current Limitations

Installer formats such as DMG, MSI, and AppImage are not part of the CLI yet. Today, `uzumaki build` produces the packaged executable and its bundled resources.
