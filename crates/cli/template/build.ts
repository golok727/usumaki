import { build } from 'rolldown';

await build({
  input: './src/index.tsx',
  output: {
    format: 'esm',
    file: 'dist/index.js',
    minify: true,
    codeSplitting: false,
  },
  external: ['uzumaki'],
});
