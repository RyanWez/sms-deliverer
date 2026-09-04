/**
 * `?raw` is a Vite import suffix, so TypeScript has to be told what it yields.
 *
 * Declared by hand rather than by referencing `vite/client` wholesale: this
 * project imports exactly one raw asset — CHANGELOG.md, read by the Changelog
 * page — and a narrow declaration cannot quietly widen the DOM and asset types
 * the rest of the app is checked against.
 */
declare module '*?raw' {
  const content: string;
  export default content;
}
