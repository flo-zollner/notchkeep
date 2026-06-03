import { describe, it, expect } from 'vitest';
import { isNewer, isPrerelease } from './semver';

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

describe('isPrerelease', () => {
  it('is false for plain releases', () => {
    expect(isPrerelease('0.3.0')).toBe(false);
    expect(isPrerelease('1.2.10')).toBe(false);
    expect(isPrerelease('v0.3.0')).toBe(false);
  });
  it('is true for rc/beta/alpha', () => {
    expect(isPrerelease('0.3.0-rc.2')).toBe(true);
    expect(isPrerelease('0.3.0-beta.1')).toBe(true);
    expect(isPrerelease('0.3.0-alpha')).toBe(true);
  });
});

describe('isNewer with prereleases', () => {
  it('release supersedes a prerelease of the same core', () => {
    expect(isNewer('0.3.0', '0.3.0-rc.2')).toBe(true);
  });
  it('a prerelease is NOT newer than its own release', () => {
    expect(isNewer('0.3.0-rc.2', '0.3.0')).toBe(false);
  });
  it('higher prerelease beats lower prerelease', () => {
    expect(isNewer('0.3.0-rc.2', '0.3.0-rc.1')).toBe(true);
    expect(isNewer('0.3.0-rc.1', '0.3.0-rc.2')).toBe(false);
  });
  it('prerelease of a higher core beats a lower release', () => {
    expect(isNewer('0.3.0-rc.1', '0.2.4')).toBe(true);
  });
  it('equal versions are not newer', () => {
    expect(isNewer('0.3.0-rc.1', '0.3.0-rc.1')).toBe(false);
  });
});
