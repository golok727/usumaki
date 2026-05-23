# Uzumaki Runtime

This package contains the built-in Uzumaki runtime module and the native host that powers app execution.

For user-facing docs, start with the main documentation site and the root repository README.

## Runtime Module Surface

Apps import runtime APIs from the built-in `uzumaki` module:

```tsx
import { Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';
```

The `uzumaki` module is provided by the runtime at execution time. It is not a normal app dependency.

## React and TypeScript

Typical app setup uses:

- `react`
- `uzumaki-react`
- `uzumaki-types`

Recommended TypeScript options:

```json
{
  "compilerOptions": {
    "types": ["uzumaki-types"],
    "jsxImportSource": "uzumaki-react"
  }
}
```

## Assets

Ship files through `bundle.resources` in `uzumaki.config.json`, then resolve them with:

```tsx
const logo = Uz.path.resource('assets/logo.svg');
```

## Useful Source Areas

- `js/runtime.ts` for the built-in module exports
- `js/window.ts` for window lifecycle and imperative creation
- `js/elements/` for low-level element classes
- `src/runtime/ts.rs` for TypeScript loading and JSX transpilation
