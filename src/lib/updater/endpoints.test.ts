import { describe, it, expect } from 'vitest';
import { updaterEndpoint } from './endpoints';

// Regression guard for the channel-endpoint bug (2026-06-03):
// the stable (default) channel had been pointed at the rolling `updater-latest`
// manifest, which (a) 404'd until that release existed and (b) hid intermediate
// stable releases whenever a prerelease was the newest build. Stable MUST resolve
// to GitHub's `/releases/latest/` (newest non-prerelease); only beta uses the
// rolling release.
describe('updaterEndpoint', () => {
  it('stable desktop → /releases/latest/ (never the rolling/prerelease manifest)', () => {
    const url = updaterEndpoint('stable', 'latest.json');
    expect(url).toBe(
      'https://github.com/flo-zollner/notchkeep/releases/latest/download/latest.json',
    );
    expect(url).not.toContain('updater-latest');
  });

  it('stable android → /releases/latest/', () => {
    const url = updaterEndpoint('stable', 'android-latest.json');
    expect(url).toBe(
      'https://github.com/flo-zollner/notchkeep/releases/latest/download/android-latest.json',
    );
    expect(url).not.toContain('updater-latest');
  });

  it('beta desktop → rolling updater-latest', () => {
    expect(updaterEndpoint('beta', 'latest.json')).toBe(
      'https://github.com/flo-zollner/notchkeep/releases/download/updater-latest/latest.json',
    );
  });

  it('beta android → rolling updater-latest', () => {
    expect(updaterEndpoint('beta', 'android-latest.json')).toBe(
      'https://github.com/flo-zollner/notchkeep/releases/download/updater-latest/android-latest.json',
    );
  });
});
