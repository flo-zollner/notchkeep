/** Splits "1.2.3-rc.4" into { core: [1,2,3], pre: ["rc","4"] }. */
function split(v: string): { core: number[]; pre: string[] } {
  const clean = v.replace(/^v/, '');
  const dash = clean.indexOf('-');
  const corePart = dash === -1 ? clean : clean.slice(0, dash);
  const prePart = dash === -1 ? '' : clean.slice(dash + 1);
  const core = corePart.split('.').map((n) => parseInt(n, 10) || 0);
  const pre = prePart === '' ? [] : prePart.split('.');
  return { core, pre };
}

/** True if the version has a prerelease suffix (e.g. "-rc.2", "-beta.1"). */
export function isPrerelease(v: string): boolean {
  return v.replace(/^v/, '').includes('-');
}

function cmpCore(a: number[], b: number[]): number {
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i] ?? 0;
    const y = b[i] ?? 0;
    if (x !== y) return x > y ? 1 : -1;
  }
  return 0;
}

/** Compares prerelease identifier lists per semver §11: a release (empty list)
 *  outranks any prerelease; numeric identifiers compare numerically; otherwise
 *  ASCII order; longer lists win when otherwise equal. */
function cmpPre(a: string[], b: string[]): number {
  if (a.length === 0 && b.length === 0) return 0;
  if (a.length === 0) return 1;  // a is a release → newer than prerelease b
  if (b.length === 0) return -1; // b is a release → newer than prerelease a
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i];
    const y = b[i];
    if (x === undefined) return -1;
    if (y === undefined) return 1;
    const nx = /^\d+$/.test(x) ? parseInt(x, 10) : null;
    const ny = /^\d+$/.test(y) ? parseInt(y, 10) : null;
    if (nx !== null && ny !== null) { if (nx !== ny) return nx > ny ? 1 : -1; }
    else if (nx !== null) return -1; // numeric < alphanumeric
    else if (ny !== null) return 1;
    else if (x !== y) return x > y ? 1 : -1;
  }
  return 0;
}

/** True if `candidate` is a strictly newer semver than `current`. */
export function isNewer(candidate: string, current: string): boolean {
  const a = split(candidate);
  const b = split(current);
  const core = cmpCore(a.core, b.core);
  if (core !== 0) return core > 0;
  return cmpPre(a.pre, b.pre) > 0;
}
