// Visit identity is independent of the indices and include closure of a compiled Document.
export function sourceKey(source) {
  const path = (source?.url || source?.path || source?.name || '').replaceAll('\\', '/');
  return /^[A-Za-z]:\//.test(path) ? path.toLowerCase() : path;
}

export function sameSource(a, b) {
  return a === b || (!!a && !!b && sourceKey(a) === sourceKey(b));
}
