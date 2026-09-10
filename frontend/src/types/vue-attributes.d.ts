// Vue's DOM attribute types do not model data-* attributes, so the e2e test
// hooks handed to naive-ui's input-props would be rejected as excess
// properties. HTMLAttributes is the base every element attribute type extends.
// This file is a module, which is what makes the block below an augmentation
// rather than a replacement of vue's own declarations.
export {};

declare module 'vue' {
  interface HTMLAttributes {
    'data-testid'?: string;
  }
}
