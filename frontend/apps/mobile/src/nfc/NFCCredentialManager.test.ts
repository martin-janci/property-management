/**
 * Unit tests for NFCCredentialManager access-log parsing.
 *
 * Regression guard: getAccessLog() runs an unguarded JSON.parse on the stored
 * blob. Because logAccessAttempt() reads the log AFTER access has already been
 * granted at the door, a corrupted blob would throw a SyntaxError and reject
 * the tap flow post-grant. The parse must be guarded: a corrupted blob is
 * treated as an empty/unavailable log rather than throwing.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

import { NFCCredentialManager } from './NFCCredentialManager';

const getItem = AsyncStorage.getItem as jest.Mock;
const setItem = AsyncStorage.setItem as jest.Mock;

const ACCESS_LOG_KEY = '@ppt/access_log';

describe('NFCCredentialManager.getAccessLog', () => {
  let manager: NFCCredentialManager;
  let warnSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    manager = new NFCCredentialManager('http://localhost:8080');
    warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    warnSpy.mockRestore();
  });

  it('returns an empty array when nothing is stored', async () => {
    getItem.mockResolvedValueOnce(null);
    await expect(manager.getAccessLog()).resolves.toEqual([]);
  });

  it('parses a valid stored log', async () => {
    const entries = [{ id: 'log-1', credentialId: 'c1', accessPointId: 'ap1', result: 'granted' }];
    getItem.mockResolvedValueOnce(JSON.stringify(entries));
    await expect(manager.getAccessLog()).resolves.toEqual(entries);
  });

  it('treats a corrupted blob as an empty log instead of throwing', async () => {
    getItem.mockResolvedValueOnce('{not valid json');
    // Must resolve (not reject) so a corrupted blob cannot break callers.
    await expect(manager.getAccessLog()).resolves.toEqual([]);
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('access log corrupted'),
      expect.anything()
    );
  });

  it('does not reject logAccessAttempt when the stored log is corrupted (post-grant safety)', async () => {
    // A corrupted blob is present when we go to record a just-granted access.
    getItem.mockResolvedValue('}}garbage[[');

    await expect(
      manager.logAccessAttempt('cred-1', 'ap-1', 'Front Door', 'granted')
    ).resolves.toBeUndefined();

    // The attempt is still persisted — the corrupted prior log is discarded and
    // replaced by a fresh single-entry array.
    expect(setItem).toHaveBeenCalledWith(ACCESS_LOG_KEY, expect.any(String));
    const [, written] = setItem.mock.calls[setItem.mock.calls.length - 1];
    const parsed = JSON.parse(written as string);
    expect(Array.isArray(parsed)).toBe(true);
    expect(parsed).toHaveLength(1);
    expect(parsed[0]).toMatchObject({
      accessPointId: 'ap-1',
      accessPointName: 'Front Door',
      result: 'granted',
    });
  });
});
