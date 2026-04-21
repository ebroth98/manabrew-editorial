/// <reference types="vite/client" />

// `unplugin-icons` with `compiler: 'raw'` exports the plain SVG string
// as the default import for any `~icons/<set>/<name>` module. Declare
// this here instead of pulling in `unplugin-icons/types/react`, which
// would type the imports as React components and shadow the string
// contract we rely on in `PointerLayer`.
declare module "~icons/*" {
  const svg: string;
  export default svg;
}
