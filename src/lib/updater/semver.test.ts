import { describe, it, expect } from 'vitest';
import { isNewer } from './semver';

describe('isNewer', () => {
  it('true when major/minor/patch greater', () => {
    expect(isNewer('0.2.3', '0.2.2')).toBe(true);
    expect(isNewer('0.3.0', '0.2.9')).toBe(true);
    expect(isNewer('1.0.0', '0.9.9')).toBe(true);
  });
  it('false when equal or older', () => {
    expect(isNewer('0.2.2', '0.2.2')).toBe(false);
    expect(isNewer('0.2.1', '0.2.2')).toBe(false);
  });
  it('ignores a leading v', () => {
    expect(isNewer('v0.2.3', '0.2.2')).toBe(true);
  });
});
