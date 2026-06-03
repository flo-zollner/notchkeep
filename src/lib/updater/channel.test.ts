import { describe, it, expect } from 'vitest';
import { filterByChannel } from './channel';

const stableUpd = { version: '0.3.0', notes: 'x' };
const rcUpd = { version: '0.3.0-rc.2', notes: 'x' };

describe('filterByChannel', () => {
  it('passes null through unchanged', () => {
    expect(filterByChannel(null, 'stable')).toBeNull();
    expect(filterByChannel(null, 'beta')).toBeNull();
  });
  it('stable drops a prerelease update', () => {
    expect(filterByChannel(rcUpd, 'stable')).toBeNull();
  });
  it('stable keeps a stable update', () => {
    expect(filterByChannel(stableUpd, 'stable')).toEqual(stableUpd);
  });
  it('beta keeps a prerelease update', () => {
    expect(filterByChannel(rcUpd, 'beta')).toEqual(rcUpd);
  });
  it('beta keeps a stable update', () => {
    expect(filterByChannel(stableUpd, 'beta')).toEqual(stableUpd);
  });
});
