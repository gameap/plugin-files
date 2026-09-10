// A dependency-free reader for the flat, generated config.yaml the installer
// writes: two-space indentation, no anchors, no flow collections, no multi-line
// scalars. Enough to assert on dotted keys, and it deliberately does not try to
// be a YAML parser.
export function readScalar(text: string, dottedKey: string): string | undefined {
  const wanted = dottedKey.split('.');
  const stack: { indent: number; key: string }[] = [];

  for (const raw of text.split(/\r?\n/)) {
    if (!raw.trim() || raw.trim().startsWith('#')) {
      continue;
    }

    const indent = raw.length - raw.trimStart().length;
    const match = /^([A-Za-z0-9_.-]+):\s*(.*)$/.exec(raw.trim());
    if (!match) {
      continue;
    }

    while (stack.length > 0 && stack[stack.length - 1].indent >= indent) {
      stack.pop();
    }

    const [, key, value] = match;
    const path = [...stack.map((entry) => entry.key), key];

    if (value === '') {
      stack.push({ indent, key });
      continue;
    }

    if (path.length === wanted.length && path.every((part, i) => part === wanted[i])) {
      return unquote(value);
    }
  }

  return undefined;
}

/** Every dotted key that has a scalar value, in file order. */
export function scalarKeys(text: string): string[] {
  const keys: string[] = [];
  const stack: { indent: number; key: string }[] = [];

  for (const raw of text.split(/\r?\n/)) {
    if (!raw.trim() || raw.trim().startsWith('#')) {
      continue;
    }

    const indent = raw.length - raw.trimStart().length;
    const match = /^([A-Za-z0-9_.-]+):\s*(.*)$/.exec(raw.trim());
    if (!match) {
      continue;
    }

    while (stack.length > 0 && stack[stack.length - 1].indent >= indent) {
      stack.pop();
    }

    const [, key, value] = match;
    if (value === '') {
      stack.push({ indent, key });
      continue;
    }

    keys.push([...stack.map((entry) => entry.key), key].join('.'));
  }

  return keys;
}

function unquote(value: string): string {
  const trimmed = value.trim();
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }

  return trimmed;
}
