import type { ReleaseChannel } from '../settings.svelte';
import { isPrerelease } from './semver';

export type UpdateInfo = { version: string; notes: string };

/**
 * Applies the release channel to an update returned by `check()`.
 * On the stable channel, prerelease updates (rc/beta/alpha) are dropped
 * (treated as "no update"). The beta channel accepts everything.
 */
export function filterByChannel(
  update: UpdateInfo | null,
  channel: ReleaseChannel,
): UpdateInfo | null {
  if (!update) return null;
  if (channel === 'stable' && isPrerelease(update.version)) return null;
  return update;
}
