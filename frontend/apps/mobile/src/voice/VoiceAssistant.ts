/**
 * Voice assistant controller for speech recognition and actions.
 *
 * Epic 49 - Story 49.2: Voice Assistant Integration
 */
import { NativeModules, Platform, Vibration } from 'react-native';
import type {
  AppAction,
  ParsedVoiceCommand,
  SiriShortcut,
  VoiceActionResult,
  VoiceAssistantConfig,
  VoiceAssistantState,
  VoiceRecognitionResult,
} from './types';
import { executeVoiceCommand, parseVoiceCommand } from './VoiceCommands';

// Native module interfaces
interface NativeSpeechRecognition {
  startListening(language: string): Promise<void>;
  stopListening(): Promise<void>;
  isAvailable(): Promise<boolean>;
}

interface NativeSiriIntents {
  registerShortcut(shortcut: SiriShortcut): Promise<void>;
  removeShortcut(id: string): Promise<void>;
  getRegisteredShortcuts(): Promise<SiriShortcut[]>;
}

interface NativeAppActions {
  registerAction(action: AppAction): Promise<void>;
  removeAction(id: string): Promise<void>;
  getRegisteredActions(): Promise<AppAction[]>;
}

const NativeSpeech: NativeSpeechRecognition | null = NativeModules.PPTSpeech ?? null;
const SiriIntents: NativeSiriIntents | null = NativeModules.PPTSiriIntents ?? null;
const AppActions: NativeAppActions | null = NativeModules.PPTAppActions ?? null;

/**
 * Voice assistant controller for hands-free interaction.
 */
export class VoiceAssistant {
  private config: VoiceAssistantConfig;
  private state: VoiceAssistantState = 'idle';
  private stateListeners: Set<(state: VoiceAssistantState) => void> = new Set();
  private resultListeners: Set<(result: VoiceRecognitionResult) => void> = new Set();
  private commandListeners: Set<(command: ParsedVoiceCommand, result: VoiceActionResult) => void> =
    new Set();

  constructor(config?: Partial<VoiceAssistantConfig>) {
    this.config = {
      language: 'en-US',
      continuous: false,
      hapticFeedback: true,
      voiceConfirmation: true,
      ...config,
    };
  }

  /**
   * Check if voice recognition is available.
   */
  async isAvailable(): Promise<boolean> {
    if (!NativeSpeech) {
      return false;
    }
    try {
      return await NativeSpeech.isAvailable();
    } catch {
      return false;
    }
  }

  /**
   * Start listening for voice commands.
   */
  async startListening(): Promise<void> {
    if (this.state !== 'idle') {
      return;
    }

    if (!NativeSpeech) {
      this.updateState('error');
      throw new Error('Speech recognition not available');
    }

    try {
      this.updateState('listening');

      if (this.config.hapticFeedback) {
        Vibration.vibrate(50);
      }

      await NativeSpeech.startListening(this.config.language);
    } catch (error) {
      this.updateState('error');
      throw error;
    }
  }

  /**
   * Stop listening.
   */
  async stopListening(): Promise<void> {
    if (this.state !== 'listening') {
      return;
    }

    if (NativeSpeech) {
      await NativeSpeech.stopListening();
    }

    this.updateState('idle');
  }

  /**
   * Process a voice recognition result.
   */
  processRecognitionResult(result: VoiceRecognitionResult): VoiceActionResult | null {
    // Notify result listeners
    for (const listener of this.resultListeners) {
      listener(result);
    }

    if (!result.isFinal) {
      return null;
    }

    this.updateState('processing');

    // Parse and execute the command
    const command = parseVoiceCommand(result.transcript);
    const actionResult = executeVoiceCommand(command);

    // Notify command listeners
    for (const listener of this.commandListeners) {
      listener(command, actionResult);
    }

    // Provide haptic feedback for result
    if (this.config.hapticFeedback) {
      Vibration.vibrate(actionResult.success ? [0, 50, 50, 50] : [0, 100, 50, 100]);
    }

    this.updateState('idle');

    return actionResult;
  }

  /**
   * Register a Siri shortcut (iOS only).
   */
  async registerSiriShortcut(shortcut: SiriShortcut): Promise<void> {
    if (Platform.OS !== 'ios' || !SiriIntents) {
      return;
    }

    await SiriIntents.registerShortcut(shortcut);
  }

  /**
   * Remove a Siri shortcut.
   */
  async removeSiriShortcut(id: string): Promise<void> {
    if (Platform.OS !== 'ios' || !SiriIntents) {
      return;
    }

    await SiriIntents.removeShortcut(id);
  }

  /**
   * Get registered Siri shortcuts.
   */
  async getSiriShortcuts(): Promise<SiriShortcut[]> {
    if (Platform.OS !== 'ios' || !SiriIntents) {
      return [];
    }

    return SiriIntents.getRegisteredShortcuts();
  }

  /**
   * Register a Google Assistant App Action (Android only).
   */
  async registerAppAction(action: AppAction): Promise<void> {
    if (Platform.OS !== 'android' || !AppActions) {
      return;
    }

    await AppActions.registerAction(action);
  }

  /**
   * Remove an App Action.
   */
  async removeAppAction(id: string): Promise<void> {
    if (Platform.OS !== 'android' || !AppActions) {
      return;
    }

    await AppActions.removeAction(id);
  }

  /**
   * Get registered App Actions.
   */
  async getAppActions(): Promise<AppAction[]> {
    if (Platform.OS !== 'android' || !AppActions) {
      return [];
    }

    return AppActions.getRegisteredActions();
  }

  /**
   * Register default voice shortcuts for the app.
   */
  async registerDefaultShortcuts(): Promise<void> {
    if (Platform.OS === 'ios') {
      await this.registerSiriShortcut({
        id: 'ppt-report-fault',
        title: 'Report Fault',
        phrase: 'Report a fault in my building',
        activityType: 'com.ppt.reportFault',
        intent: 'report_fault',
      });

      await this.registerSiriShortcut({
        id: 'ppt-announcements',
        title: 'Check Announcements',
        phrase: 'Show building announcements',
        activityType: 'com.ppt.announcements',
        intent: 'view_announcements',
      });

      await this.registerSiriShortcut({
        id: 'ppt-vote',
        title: 'Vote',
        phrase: 'Open voting',
        activityType: 'com.ppt.vote',
        intent: 'cast_vote',
      });
    } else if (Platform.OS === 'android') {
      await this.registerAppAction({
        id: 'ppt-report-fault',
        name: 'Report Fault',
        intentAction: 'com.ppt.action.REPORT_FAULT',
        builtInIntent: 'actions.intent.CREATE_THING',
        parameters: { 'thing.name': 'fault' },
      });

      await this.registerAppAction({
        id: 'ppt-announcements',
        name: 'View Announcements',
        intentAction: 'com.ppt.action.VIEW_ANNOUNCEMENTS',
        builtInIntent: 'actions.intent.OPEN_APP_FEATURE',
        parameters: { 'feature.name': 'announcements' },
      });
    }
  }

  /**
   * Get current state.
   */
  getState(): VoiceAssistantState {
    return this.state;
  }

  /**
   * Subscribe to state changes.
   */
  onStateChange(listener: (state: VoiceAssistantState) => void): () => void {
    this.stateListeners.add(listener);
    return () => this.stateListeners.delete(listener);
  }

  /**
   * Subscribe to recognition results.
   */
  onRecognitionResult(listener: (result: VoiceRecognitionResult) => void): () => void {
    this.resultListeners.add(listener);
    return () => this.resultListeners.delete(listener);
  }

  /**
   * Subscribe to command execution.
   */
  onCommand(
    listener: (command: ParsedVoiceCommand, result: VoiceActionResult) => void
  ): () => void {
    this.commandListeners.add(listener);
    return () => this.commandListeners.delete(listener);
  }

  /**
   * Update configuration.
   */
  updateConfig(config: Partial<VoiceAssistantConfig>): void {
    this.config = { ...this.config, ...config };
  }

  private updateState(state: VoiceAssistantState): void {
    this.state = state;
    for (const listener of this.stateListeners) {
      listener(state);
    }
  }
}
