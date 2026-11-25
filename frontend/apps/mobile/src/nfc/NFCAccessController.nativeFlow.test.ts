/**
 * Native-module flow tests for `NFCAccessController`.
 *
 * Regression guard for `code-review-mobile-rn-nfc-untested`. The sibling
 * characterization suite (`NFCAccessController.test.ts`) deliberately runs with
 * the native `PPTNFC` / `PPTWallet` modules ABSENT (`NativeModules.PPTNFC` is
 * undefined in jest), so it only exercises the pure access-decision core and
 * explicitly notes that the hardware-facing half of the controller is never
 * reached. That left the entire native NFC flow uncovered:
 *
 *   - `initialize()` hardware probe (isSupported / isEnabled -> ready / disabled
 *     / error),
 *   - the session lifecycle (`startAccessSession` credential selection +
 *     `stopAccessSession`),
 *   - the credential-transmit path in `handleTap`, including the two
 *     `hardware_error` fallbacks (transmit returns false / transmit throws),
 *   - the Apple Wallet integration (`addToWallet` / `removeFromWallet` /
 *     `isInWallet`), and
 *   - the state / tap / result listener fan-out.
 *
 * The controller captures its native handles at module-eval time
 * (`const NativeNFC = NativeModules.PPTNFC ?? null`), so the jest mocks are
 * attached to `NativeModules` BEFORE the module is `require()`d. The mock
 * methods are stable `jest.fn()`s whose behavior is reconfigured per-test with
 * `mockResolvedValue`, so the captured handles never need to change.
 */
import { NativeModules, Platform, Vibration } from 'react-native';

import type { NFCCredential, NFCHardwareState } from './types';

interface NativeNFCMock {
  isSupported: jest.Mock;
  isEnabled: jest.Mock;
  startSession: jest.Mock;
  stopSession: jest.Mock;
  transmitCredential: jest.Mock;
}

interface NativeWalletMock {
  isAvailable: jest.Mock;
  addPass: jest.Mock;
  removePass: jest.Mock;
  getInstalledPasses: jest.Mock;
}

const nfc: NativeNFCMock = {
  isSupported: jest.fn(),
  isEnabled: jest.fn(),
  startSession: jest.fn(),
  stopSession: jest.fn(),
  transmitCredential: jest.fn(),
};

const wallet: NativeWalletMock = {
  isAvailable: jest.fn(),
  addPass: jest.fn(),
  removePass: jest.fn(),
  getInstalledPasses: jest.fn(),
};

// Attach the native modules BEFORE the controller module is evaluated so its
// module-scope `NativeModules.PPTNFC ?? null` capture picks up the mocks.
(NativeModules as unknown as Record<string, unknown>).PPTNFC = nfc;
(NativeModules as unknown as Record<string, unknown>).PPTWallet = wallet;

// Required (not statically imported) AFTER the native modules are attached.
const { NFCAccessController } =
  require('./NFCAccessController') as typeof import('./NFCAccessController');

const API_BASE_URL = 'http://localhost:8080';
const ORIGINAL_OS = Platform.OS;

/** Force `Platform.OS` for the iOS-gated Wallet branches. */
function setPlatformOS(os: typeof Platform.OS): void {
  (Platform as unknown as { OS: string }).OS = os;
}

function makeCredential(overrides: Partial<NFCCredential> = {}): NFCCredential {
  return {
    id: 'cred-1',
    buildingId: 'building-1',
    buildingName: 'Building 1',
    userId: 'user-1',
    accessLevel: 'resident',
    accessPoints: [{ id: 'ap-1', name: 'Main Entrance', type: 'main_entrance' }],
    validFrom: '2020-01-01T00:00:00.000Z',
    validUntil: '2999-01-01T00:00:00.000Z',
    status: 'active',
    usageCount: 0,
    encryptedData: 'ZW5jcnlwdGVk',
    ...overrides,
  };
}

type Controller = InstanceType<typeof NFCAccessController>;

/**
 * Build a controller, stub its credential manager so decisions are isolated
 * from storage/network, and (by default) drive `initialize()` to the `ready`
 * state with the native module reporting supported + enabled.
 */
async function makeReadyController(
  creds: NFCCredential[] = [],
  { initialize = true }: { initialize?: boolean } = {}
): Promise<{ controller: Controller; manager: ReturnType<Controller['getCredentialManager']> }> {
  nfc.isSupported.mockResolvedValue(true);
  nfc.isEnabled.mockResolvedValue(true);

  const controller = new NFCAccessController(API_BASE_URL);
  const manager = controller.getCredentialManager();

  // Keep initialize()/decision paths off real storage.
  jest.spyOn(manager, 'initialize').mockResolvedValue(undefined);
  jest.spyOn(manager, 'getActiveCredentials').mockReturnValue(creds);
  jest
    .spyOn(manager, 'getCredentialById')
    .mockImplementation((id: string) => creds.find((c) => c.id === id));
  jest.spyOn(manager, 'logAccessAttempt').mockResolvedValue(undefined);
  jest.spyOn(manager, 'updateCredentialUsage').mockResolvedValue(undefined);

  if (initialize) {
    await controller.initialize(null);
  }
  return { controller, manager };
}

/** Seed the private active credential id (normally set by startAccessSession). */
function seedActiveCredential(controller: Controller, id: string | null): void {
  (controller as unknown as { activeCredentialId: string | null }).activeCredentialId = id;
}

beforeEach(() => {
  for (const fn of Object.values(nfc)) {
    fn.mockReset();
  }
  for (const fn of Object.values(wallet)) {
    fn.mockReset();
  }
  jest.spyOn(Vibration, 'vibrate').mockImplementation(() => {});
});

afterEach(() => {
  jest.restoreAllMocks();
  setPlatformOS(ORIGINAL_OS);
});

describe('NFCAccessController.initialize — hardware probe (native module present)', () => {
  it('reports ready when the hardware is supported and enabled', async () => {
    nfc.isSupported.mockResolvedValue(true);
    nfc.isEnabled.mockResolvedValue(true);

    const controller = new NFCAccessController(API_BASE_URL);
    jest.spyOn(controller.getCredentialManager(), 'initialize').mockResolvedValue(undefined);

    await controller.initialize('token-123');

    expect(controller.getState()).toBe('ready');
    expect(controller.isSupported()).toBe(true);
    expect(controller.isReady()).toBe(true);
    expect(nfc.isSupported).toHaveBeenCalledTimes(1);
    expect(nfc.isEnabled).toHaveBeenCalledTimes(1);
  });

  it('reports disabled when supported but the NFC radio is off', async () => {
    nfc.isSupported.mockResolvedValue(true);
    nfc.isEnabled.mockResolvedValue(false);

    const controller = new NFCAccessController(API_BASE_URL);
    jest.spyOn(controller.getCredentialManager(), 'initialize').mockResolvedValue(undefined);

    await controller.initialize(null);

    expect(controller.getState()).toBe('disabled');
    expect(controller.isReady()).toBe(false);
    // Supported is true even while disabled (state !== 'unsupported').
    expect(controller.isSupported()).toBe(true);
    expect(controller.getStateDescription()).toBe('NFC is disabled. Enable it in Settings');
  });

  it('reports unsupported when the hardware probe says the device has no NFC', async () => {
    nfc.isSupported.mockResolvedValue(false);

    const controller = new NFCAccessController(API_BASE_URL);
    jest.spyOn(controller.getCredentialManager(), 'initialize').mockResolvedValue(undefined);

    await controller.initialize(null);

    expect(controller.getState()).toBe('unsupported');
    expect(nfc.isEnabled).not.toHaveBeenCalled();
  });

  it('reports error when the native probe rejects', async () => {
    nfc.isSupported.mockRejectedValue(new Error('native boom'));

    const controller = new NFCAccessController(API_BASE_URL);
    jest.spyOn(controller.getCredentialManager(), 'initialize').mockResolvedValue(undefined);

    await controller.initialize(null);

    expect(controller.getState()).toBe('error');
    expect(controller.getStateDescription()).toBe('NFC error. Please try again');
  });
});

describe('NFCAccessController session lifecycle', () => {
  it('starts a session for an explicitly named credential: opens native session, buzzes, goes reading', async () => {
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);

    await controller.startAccessSession('cred-1');

    expect(nfc.startSession).toHaveBeenCalledTimes(1);
    expect(controller.getState()).toBe('reading');
    expect(Vibration.vibrate).toHaveBeenCalledWith(50);
    // The session must remember which credential is live for handleTap().
    expect(
      (controller as unknown as { activeCredentialId: string | null }).activeCredentialId
    ).toBe('cred-1');
  });

  it('auto-selects the sole active credential when none is named', async () => {
    const cred = makeCredential({ id: 'only-cred' });
    const { controller } = await makeReadyController([cred]);

    await controller.startAccessSession();

    expect(nfc.startSession).toHaveBeenCalledTimes(1);
    expect(
      (controller as unknown as { activeCredentialId: string | null }).activeCredentialId
    ).toBe('only-cred');
  });

  it('refuses to guess when several active credentials exist', async () => {
    const { controller } = await makeReadyController([
      makeCredential({ id: 'a' }),
      makeCredential({ id: 'b' }),
    ]);

    await expect(controller.startAccessSession()).rejects.toThrow(
      'Multiple credentials available, please specify one'
    );
    expect(nfc.startSession).not.toHaveBeenCalled();
  });

  it('throws when there is no active credential to present', async () => {
    const { controller } = await makeReadyController([]);

    await expect(controller.startAccessSession()).rejects.toThrow('No active credential found');
    expect(nfc.startSession).not.toHaveBeenCalled();
  });

  it('throws when the controller is not in the ready state', async () => {
    const cred = makeCredential();
    // Do not initialize -> state stays 'unsupported'.
    const { controller } = await makeReadyController([cred], { initialize: false });

    await expect(controller.startAccessSession('cred-1')).rejects.toThrow('NFC not ready');
    expect(nfc.startSession).not.toHaveBeenCalled();
  });

  it('transitions to error and rethrows when the native session fails to open', async () => {
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);
    nfc.startSession.mockRejectedValue(new Error('session failed'));

    await expect(controller.startAccessSession('cred-1')).rejects.toThrow('session failed');
    expect(controller.getState()).toBe('error');
  });

  it('stops an active session: closes the native session and returns to ready', async () => {
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);
    await controller.startAccessSession('cred-1');
    expect(controller.getState()).toBe('reading');

    await controller.stopAccessSession();

    expect(nfc.stopSession).toHaveBeenCalledTimes(1);
    expect(controller.getState()).toBe('ready');
    expect(
      (controller as unknown as { activeCredentialId: string | null }).activeCredentialId
    ).toBeNull();
  });

  it('does not touch the native session on stop when idle (state ready)', async () => {
    const { controller } = await makeReadyController([]);

    await controller.stopAccessSession();

    expect(nfc.stopSession).not.toHaveBeenCalled();
    expect(controller.getState()).toBe('ready');
  });
});

describe('NFCAccessController.handleTap — credential transmit (native module present)', () => {
  it('transmits the encrypted credential and grants when transmit succeeds', async () => {
    const cred = makeCredential();
    const { controller, manager } = await makeReadyController([cred]);
    seedActiveCredential(controller, 'cred-1');
    nfc.transmitCredential.mockResolvedValue(true);

    const result = await controller.handleTap('ap-1', 'Main Entrance');

    expect(result.granted).toBe(true);
    expect(result.denialReason).toBeUndefined();
    expect(nfc.transmitCredential).toHaveBeenCalledWith(cred.encryptedData);
    // Granted tap is logged as granted, bumps usage, and buzzes the success pattern.
    expect(manager.logAccessAttempt).toHaveBeenCalledWith(
      'cred-1',
      'ap-1',
      'Main Entrance',
      'granted',
      undefined
    );
    expect(manager.updateCredentialUsage).toHaveBeenCalledWith('cred-1');
    expect(Vibration.vibrate).toHaveBeenCalledWith([0, 100]);
    // The session returns to ready after the tap.
    expect(controller.getState()).toBe('ready');
  });

  it('flips an authorized tap to hardware_error when the native transmit returns false', async () => {
    const cred = makeCredential();
    const { controller, manager } = await makeReadyController([cred]);
    seedActiveCredential(controller, 'cred-1');
    nfc.transmitCredential.mockResolvedValue(false);

    const result = await controller.handleTap('ap-1', 'Main Entrance');

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('hardware_error');
    expect(result.message).toBe('Failed to transmit credential');
    // A transmit failure is a denied attempt: logged denied, usage NOT bumped.
    expect(manager.logAccessAttempt).toHaveBeenCalledWith(
      'cred-1',
      'ap-1',
      'Main Entrance',
      'denied',
      'hardware_error'
    );
    expect(manager.updateCredentialUsage).not.toHaveBeenCalled();
    // Denied -> the error-buzz pattern.
    expect(Vibration.vibrate).toHaveBeenCalledWith([0, 50, 50, 50, 50, 50]);
  });

  it('flips an authorized tap to hardware_error when the native transmit throws', async () => {
    const cred = makeCredential();
    const { controller, manager } = await makeReadyController([cred]);
    seedActiveCredential(controller, 'cred-1');
    nfc.transmitCredential.mockRejectedValue(new Error('rf collision'));

    const result = await controller.handleTap('ap-1', 'Main Entrance');

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('hardware_error');
    expect(result.message).toBe('NFC transmission error');
    expect(manager.updateCredentialUsage).not.toHaveBeenCalled();
  });

  it('never transmits when the authorization decision already denied the tap', async () => {
    const cred = makeCredential({ status: 'revoked' });
    const { controller } = await makeReadyController([cred]);
    seedActiveCredential(controller, 'cred-1');

    const result = await controller.handleTap('ap-1', 'Main Entrance');

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('credential_revoked');
    // A denial short-circuits before the radio is ever driven.
    expect(nfc.transmitCredential).not.toHaveBeenCalled();
  });

  it('notifies tap and result listeners for a completed tap', async () => {
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);
    seedActiveCredential(controller, 'cred-1');
    nfc.transmitCredential.mockResolvedValue(true);

    const tapEvents: unknown[] = [];
    const results: unknown[] = [];
    controller.onTap((e) => tapEvents.push(e));
    controller.onAccessResult((r) => results.push(r));

    await controller.handleTap('ap-1', 'Main Entrance');

    expect(tapEvents).toHaveLength(1);
    expect(tapEvents[0]).toMatchObject({
      type: 'write',
      credentialId: 'cred-1',
      accessPointId: 'ap-1',
    });
    expect(results).toHaveLength(1);
    expect(results[0]).toMatchObject({ granted: true, accessPointName: 'Main Entrance' });
  });
});

describe('NFCAccessController state listeners', () => {
  it('emits every state transition and stops after unsubscribe', async () => {
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);

    const states: NFCHardwareState[] = [];
    const unsubscribe = controller.onStateChange((s) => states.push(s));

    await controller.startAccessSession('cred-1'); // ready -> reading
    await controller.stopAccessSession(); // reading -> ready

    expect(states).toEqual(['reading', 'ready']);

    unsubscribe();
    await controller.startAccessSession('cred-1');
    // No further events after unsubscribe.
    expect(states).toEqual(['reading', 'ready']);
  });
});

describe('NFCAccessController Apple Wallet integration (iOS)', () => {
  beforeEach(() => {
    setPlatformOS('ios');
  });

  it('adds a credential as a wallet pass carrying its serial + NFC payload', async () => {
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);
    wallet.isAvailable.mockResolvedValue(true);
    wallet.addPass.mockResolvedValue(true);

    const added = await controller.addToWallet('cred-1');

    expect(added).toBe(true);
    expect(wallet.addPass).toHaveBeenCalledTimes(1);
    const passData = JSON.parse(wallet.addPass.mock.calls[0][0] as string);
    expect(passData).toMatchObject({
      serialNumber: 'cred-1',
      organizationName: 'Property Management',
      style: 'generic',
      nfcData: cred.encryptedData,
    });
    expect(passData.description).toBe('Building 1 Access');
  });

  it('refuses to add a credential that is not in the cache', async () => {
    const { controller } = await makeReadyController([]);
    wallet.isAvailable.mockResolvedValue(true);

    await expect(controller.addToWallet('missing')).resolves.toBe(false);
    expect(wallet.addPass).not.toHaveBeenCalled();
  });

  it('refuses to add when Wallet is unavailable on the device', async () => {
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);
    wallet.isAvailable.mockResolvedValue(false);

    await expect(controller.addToWallet('cred-1')).resolves.toBe(false);
    expect(wallet.addPass).not.toHaveBeenCalled();
  });

  it('refuses to add a credential with empty encrypted data', async () => {
    const cred = makeCredential({ encryptedData: '' });
    const { controller } = await makeReadyController([cred]);
    wallet.isAvailable.mockResolvedValue(true);
    const errorSpy = jest.spyOn(console, 'error').mockImplementation(() => {});

    await expect(controller.addToWallet('cred-1')).resolves.toBe(false);
    expect(wallet.addPass).not.toHaveBeenCalled();
    expect(errorSpy).toHaveBeenCalled();
  });

  it('removes a pass and reports the native result', async () => {
    const { controller } = await makeReadyController([]);
    wallet.removePass.mockResolvedValue(true);

    await expect(controller.removeFromWallet('cred-1')).resolves.toBe(true);
    expect(wallet.removePass).toHaveBeenCalledWith('cred-1');
  });

  it('reports wallet membership from the installed-pass list', async () => {
    const { controller } = await makeReadyController([]);
    wallet.getInstalledPasses.mockResolvedValue(['cred-1', 'cred-9']);

    await expect(controller.isInWallet('cred-1')).resolves.toBe(true);
    await expect(controller.isInWallet('cred-404')).resolves.toBe(false);
  });

  it('is a no-op that returns false off iOS', async () => {
    setPlatformOS('android');
    const cred = makeCredential();
    const { controller } = await makeReadyController([cred]);

    await expect(controller.addToWallet('cred-1')).resolves.toBe(false);
    await expect(controller.removeFromWallet('cred-1')).resolves.toBe(false);
    await expect(controller.isInWallet('cred-1')).resolves.toBe(false);
    expect(wallet.addPass).not.toHaveBeenCalled();
    expect(wallet.removePass).not.toHaveBeenCalled();
    expect(wallet.getInstalledPasses).not.toHaveBeenCalled();
  });
});
