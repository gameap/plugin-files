/** `/…`, `\…`, `X:\…` or `X:/…`, exactly like is_absolute_node_path. */
export function isAbsoluteNodePath(path: string): boolean {
  return /^[/\\]/.test(path) || /^[A-Za-z]:[/\\]/.test(path);
}

// A copy of the plugin's join_node_path, separators included: it swaps the
// separator only at the join point and leaves whatever is inside `relative`
// alone, so a home_dir the plugin wrote compares equal to one built here.
export function joinNodePath(workPath: string, relative: string): string {
  if (isAbsoluteNodePath(relative)) {
    return relative;
  }

  const separator = workPath.includes('\\') ? '\\' : '/';
  const base = workPath.replace(/[/\\]+$/, '');
  const rest = relative.replace(/^[/\\]+/, '');

  if (rest === '') {
    return base === '' ? workPath : base;
  }

  return `${base}${separator}${rest}`;
}
