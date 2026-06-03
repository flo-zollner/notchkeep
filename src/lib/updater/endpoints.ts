import type { ReleaseChannel } from '../settings.svelte';

const RELEASES = 'https://github.com/flo-zollner/notchkeep/releases';

/** Manifest filename per platform. */
export type ManifestFile = 'latest.json' | 'android-latest.json';

/**
 * Resolves the updater manifest endpoint for a release channel.
 *
 * - `stable` → GitHub's `/releases/latest/` which **always serves the newest
 *   non-prerelease**. This is the proven, regression-free source: GitHub excludes
 *   prereleases from `latest`, so a stable user is always offered the newest
 *   stable, even when a release candidate is the newest build overall.
 * - `beta` → the rolling `updater-latest` release, which the release pipeline
 *   keeps pointed at the newest build *including* prereleases.
 *
 * Why per-channel endpoints (not one shared endpoint + client filter): pointing
 * the default/stable channel at a prerelease-inclusive manifest and filtering
 * client-side meant a stable user could miss an intermediate stable while an RC
 * was the newest build — and broke the check entirely until the rolling release
 * existed. The endpoint *is* the channel.
 */
export function updaterEndpoint(channel: ReleaseChannel, file: ManifestFile): string {
  return channel === 'beta'
    ? `${RELEASES}/download/updater-latest/${file}`
    : `${RELEASES}/latest/download/${file}`;
}
