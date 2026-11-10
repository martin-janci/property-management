/**
 * NFC access controller for building entry.
 *
 * Epic 49 - Story 49.4: NFC Building Access
 */
import { NativeModules, Platform, Vibration } from 'react-native';

import { NFCCredentialManager } from './NFCCredentialManager';
import type {
  AccessAttemptResult,
  AccessDenialReason,
  AccessPoint,
  NFCCredential,
  NFCHardwareState,
  NFCTapEvent,
  WalletPass,
} from './types';

// Native module interfaces
interface NativeNFCModule {
  isSupported(): Promise<boolean>;
  isEnabled(): Promise<boolean>;
  startSession(): Promise<void>;
  stopSession(): Promise<void>;
  transmitCredential(data: string): Promise<boolean>;
}

interface NativeWalletModule {
  isAvailable(): Promise<boolean>;
  addPass(passData: string): Promise<boolean>;
  removePass(passId: string): Promise<boolean>;
  getInstalledPasses(): Promise<string[]>;
}

const NativeNFC: NativeNFCModule | null = NativeModules.PPTNFC ?? null;
const NativeWallet: NativeWalletModule | null = NativeModules.PPTWallet ?? null;

/**
 * Controller for NFC-based building access.
 */
export class NFCAccessController {
  private credentialManager: NFCCredentialManager;
  private state: NFCHardwareState = 'unsupported';
  private stateListeners: Set<(state: NFCHardwareState) => void> = new Set();
  private tapListeners: Set<(event: NFCTapEvent) => void> = new Set();
  private resultListeners: Set<(result: AccessAttemptResult) => void> = new Set();
  private activeCredentialId: string | null = null;

  constructor(apiBaseUrl: string) {
    this.credentialManager = new NFCCredentialManager(apiBaseUrl);
  }

  /**
   * Initialize the NFC controller.
   */
  async initialize(authToken: string | null): Promise<void> {
    this.credentialManager.setAuthToken(authToken);
    await this.credentialManager.initialize();

    // Check hardware support
    if (!NativeNFC) {
      this.updateState('unsupported');
      return;
    }

    try {
      const supported = await NativeNFC.isSupported();
      if (!supported) {
        this.updateState('unsupported');
        return;
      }

      const enabled = await NativeNFC.isEnabled();
      this.updateState(enabled ? 'ready' : 'disabled');
    } catch {
      this.updateState('error');
    }
  }

  /**
   * Check if NFC is supported.
   */
  isSupported(): boolean {
    return this.state !== 'unsupported';
  }

  /**
   * Check if NFC is ready for use.
   */
  isReady(): boolean {
    return this.state === 'ready';
  }

  /**
   * Get current hardware state.
   */
  getState(): NFCHardwareState {
    return this.state;
  }

  /**
   * Get credential manager.
   */
  getCredentialManager(): NFCCredentialManager {
    return this.credentialManager;
  }

  /**
   * Start NFC session for building access.
   */
  async startAccessSession(credentialId?: string): Promise<void> {
    if (this.state !== 'ready') {
      throw new Error('NFC not ready');
    }

    // Use specified credential or find active one
    let credential: NFCCredential | undefined;
    if (credentialId) {
      credential = this.credentialManager.getCredentialById(credentialId);
    } else {
      const activeCredentials = this.credentialManager.getActiveCredentials();
      if (activeCredentials.length === 1) {
        credential = activeCredentials[0];
      } else if (activeCredentials.length > 1) {
        throw new Error('Multiple credentials available, please specify one');
      }
    }

    if (!credential) {
      throw new Error('No active credential found');
    }

    this.activeCredentialId = credential.id;

    try {
      if (NativeNFC) {
        await NativeNFC.startSession();
      }
      this.updateState('reading');

      // Provide haptic feedback
      Vibration.vibrate(50);
    } catch (error) {
      this.updateState('error');
      throw error;
    }
  }

  /**
   * Stop NFC session.
   */
  async stopAccessSession(): Promise<void> {
    if (NativeNFC && (this.state === 'reading' || this.state === 'transmitting')) {
      await NativeNFC.stopSession();
    }
    this.activeCredentialId = null;
    this.updateState('ready');
  }

  /**
   * Handle NFC tap detection (called from native module).
   */
  async handleTap(accessPointId: string, accessPointName: string): Promise<AccessAttemptResult> {
    if (!this.activeCredentialId) {
      return {
        granted: false,
        timestamp: new Date().toISOString(),
        accessPointId,
        accessPointName,
        denialReason: 'invalid_credential',
        message: 'No active credential',
      };
    }

    const credential = this.credentialManager.getCredentialById(this.activeCredentialId);
    if (!credential) {
      return {
        granted: false,
        timestamp: new Date().toISOString(),
        accessPointId,
        accessPointName,
        denialReason: 'invalid_credential',
        message: 'Credential not found',
      };
    }

    // Validate credential
    const validationResult = this.validateAccess(credential, accessPointId);

    if (validationResult.granted) {
      // Transmit credential
      this.updateState('transmitting');

      try {
        if (NativeNFC) {
          const success = await NativeNFC.transmitCredential(credential.encryptedData);
          if (!success) {
            validationResult.granted = false;
            validationResult.denialReason = 'hardware_error';
            validationResult.message = 'Failed to transmit credential';
          }
        }
      } catch {
        validationResult.granted = false;
        validationResult.denialReason = 'hardware_error';
        validationResult.message = 'NFC transmission error';
      }
    }

    // Log the attempt
    await this.credentialManager.logAccessAttempt(
      credential.id,
      accessPointId,
      accessPointName,
      validationResult.granted ? 'granted' : 'denied',
      validationResult.denialReason
    );

    // Update usage if granted
    if (validationResult.granted) {
      await this.credentialManager.updateCredentialUsage(credential.id);
    }

    // Provide haptic feedback
    Vibration.vibrate(validationResult.granted ? [0, 100] : [0, 50, 50, 50, 50, 50]);

    // Notify listeners
    const tapEvent: NFCTapEvent = {
      type: 'write',
      credentialId: credential.id,
      accessPointId,
      timestamp: new Date().toISOString(),
    };
    for (const listener of this.tapListeners) {
      listener(tapEvent);
    }
    for (const listener of this.resultListeners) {
      listener(validationResult);
    }

    this.updateState('ready');

    return validationResult;
  }

  /**
   * Validate access for a credential and access point.
   */
  private validateAccess(credential: NFCCredential, accessPointId: string): AccessAttemptResult {
    const now = new Date();
    const timestamp = now.toISOString();

    // Check credential status
    if (credential.status !== 'active') {
      // Map credential status to known denial reasons
      const statusToDenialReason: Record<string, AccessDenialReason> = {
        suspended: 'credential_suspended',
        revoked: 'credential_revoked',
        expired: 'credential_expired',
      };
      const denialReason: AccessDenialReason =
        statusToDenialReason[credential.status] ?? 'invalid_credential';

      return {
        granted: false,
        timestamp,
        accessPointId,
        accessPointName: this.getAccessPointName(credential, accessPointId),
        denialReason,
        message: `Credential is ${credential.status}`,
      };
    }

    // Check expiry
    if (new Date(credential.validUntil) < now) {
      return {
        granted: false,
        timestamp,
        accessPointId,
        accessPointName: this.getAccessPointName(credential, accessPointId),
        denialReason: 'credential_expired',
        message: 'Credential has expired',
      };
    }

    // Check access point
    const accessPoint = credential.accessPoints.find((ap) => ap.id === accessPointId);
    if (!accessPoint) {
      return {
        granted: false,
        timestamp,
        accessPointId,
        accessPointName: 'Unknown',
        denialReason: 'access_point_not_allowed',
        message: 'Access point not authorized',
      };
    }

    // Check time restrictions
    if (accessPoint.timeRestrictions && accessPoint.timeRestrictions.length > 0) {
      const currentDay = now.getDay();
      // Convert current time to minutes since midnight for numeric comparison
      const currentMinutes = now.getHours() * 60 + now.getMinutes();

      const parseTimeToMinutes = (timeStr: string): number => {
        const [hours, minutes] = timeStr.split(':').map(Number);
        return hours * 60 + minutes;
      };

      const isAllowed = accessPoint.timeRestrictions.some((restriction) => {
        const startMinutes = parseTimeToMinutes(restriction.startTime);
        const endMinutes = parseTimeToMinutes(restriction.endTime);

        // A restriction's `days` list names the calendar day on which the
        // window OPENS. For a window that crosses midnight (e.g. 23:00–01:00)
        // the two halves fall on different calendar days, so each half must be
        // matched against the day it actually belongs to — not blanket against
        // the tap's current day. Otherwise Mon 23:00–01:00 wrongly denies the
        // Tue 00:00–01:00 tail and spuriously grants Mon 00:00–01:00.
        if (endMinutes < startMinutes) {
          // Evening half (>= start, before midnight): the tap day is itself a
          // start day.
          const inEveningHalf =
            currentMinutes >= startMinutes && restriction.days.includes(currentDay);
          // Early-morning half (<= end, after midnight): this belongs to the
          // window that opened on the PREVIOUS calendar day.
          const previousDay = (currentDay + 6) % 7;
          const inMorningHalf =
            currentMinutes <= endMinutes && restriction.days.includes(previousDay);
          return inEveningHalf || inMorningHalf;
        }

        // Same-day window: the tap day must be a start day and the time must
        // fall within [start, end].
        if (!restriction.days.includes(currentDay)) {
          return false;
        }
        return currentMinutes >= startMinutes && currentMinutes <= endMinutes;
      });

      if (!isAllowed) {
        return {
          granted: false,
          timestamp,
          accessPointId,
          accessPointName: accessPoint.name,
          denialReason: 'time_restriction',
          message: 'Access not allowed at this time',
        };
      }
    }

    // Access granted
    return {
      granted: true,
      timestamp,
      accessPointId,
      accessPointName: accessPoint.name,
      message: `Access granted: ${accessPoint.name}`,
    };
  }

  /**
   * Get access point name from credential.
   */
  private getAccessPointName(credential: NFCCredential, accessPointId: string): string {
    const accessPoint = credential.accessPoints.find((ap) => ap.id === accessPointId);
    return accessPoint?.name ?? 'Unknown';
  }

  /**
   * Add credential to Apple Wallet (iOS only).
   */
  async addToWallet(credentialId: string): Promise<boolean> {
    if (Platform.OS !== 'ios' || !NativeWallet) {
      return false;
    }

    const credential = this.credentialManager.getCredentialById(credentialId);
    if (!credential) {
      return false;
    }

    const isAvailable = await NativeWallet.isAvailable();
    if (!isAvailable) {
      return false;
    }

    // Validate encryptedData before creating wallet pass
    if (!credential.encryptedData || credential.encryptedData.length === 0) {
      console.error('Cannot add to wallet: credential has no encrypted data');
      return false;
    }

    const passData: WalletPass = {
      // passTypeIdentifier must be configured in Apple Developer Portal
      // and match the provisioning profile. See docs/ios-wallet-setup.md
      passTypeIdentifier: 'pass.bit.two.three.ppt.access',
      serialNumber: credential.id,
      description: `${credential.buildingName} Access`,
      organizationName: 'Property Management',
      style: 'generic',
      nfcData: credential.encryptedData,
    };

    return NativeWallet.addPass(JSON.stringify(passData));
  }

  /**
   * Remove credential from Apple Wallet.
   */
  async removeFromWallet(credentialId: string): Promise<boolean> {
    if (Platform.OS !== 'ios' || !NativeWallet) {
      return false;
    }

    return NativeWallet.removePass(credentialId);
  }

  /**
   * Check if credential is in Apple Wallet.
   */
  async isInWallet(credentialId: string): Promise<boolean> {
    if (Platform.OS !== 'ios' || !NativeWallet) {
      return false;
    }

    const passes = await NativeWallet.getInstalledPasses();
    return passes.includes(credentialId);
  }

  /**
   * Subscribe to state changes.
   */
  onStateChange(listener: (state: NFCHardwareState) => void): () => void {
    this.stateListeners.add(listener);
    return () => this.stateListeners.delete(listener);
  }

  /**
   * Subscribe to tap events.
   */
  onTap(listener: (event: NFCTapEvent) => void): () => void {
    this.tapListeners.add(listener);
    return () => this.tapListeners.delete(listener);
  }

  /**
   * Subscribe to access results.
   */
  onAccessResult(listener: (result: AccessAttemptResult) => void): () => void {
    this.resultListeners.add(listener);
    return () => this.resultListeners.delete(listener);
  }

  /**
   * Update state and notify listeners.
   */
  private updateState(state: NFCHardwareState): void {
    this.state = state;
    for (const listener of this.stateListeners) {
      listener(state);
    }
  }

  /**
   * Get user-friendly state description.
   */
  getStateDescription(): string {
    switch (this.state) {
      case 'unsupported':
        return 'NFC is not supported on this device';
      case 'disabled':
        return 'NFC is disabled. Enable it in Settings';
      case 'ready':
        return 'Ready for building access';
      case 'reading':
        return 'Hold near the reader...';
      case 'transmitting':
        return 'Transmitting credential...';
      case 'error':
        return 'NFC error. Please try again';
    }
  }

  /**
   * Get access points for a credential.
   */
  getAccessPointsForCredential(credentialId: string): AccessPoint[] {
    const credential = this.credentialManager.getCredentialById(credentialId);
    return credential?.accessPoints ?? [];
  }
}
