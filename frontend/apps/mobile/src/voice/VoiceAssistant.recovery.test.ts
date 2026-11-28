/**
 * VoiceAssistant — error-recovery regression test
 * (code-review-mobile-rn-voiceassistant-error-deadend).
 *
 * The voice-assistant controller drives a small state machine
 * (`idle → listening → processing → idle`, with an `error` branch). The
 * `error` state used to be a DEAD-END: once `startListening()` failed — which
 * happens whenever the native speech module is absent (`NativeModules.PPTSpeech`
 * is `null`, e.g. on a device without recognition, in a web/Expo-Go context, or
 * when the native call throws) — the controller set `state = 'error'` and there
 * was no public way back:
 *
 *   - `startListening()` early-returns unless `state === 'idle'`  → a retry no-ops.
 *   - `stopListening()`  early-returns unless `state === 'listening'` → useless here.
 *   - `updateState()` is private.
 *
 * So the UI (which subscribes via `onStateChange`) would render an error with no
 * retry / no way forward, leaving the user permanently stuck on a bricked
 * assistant. This test locks the recovery contract:
 *
 *   1. A failed start lands in `error` (precondition).
 *   2. `reset()` brings the controller back to `idle` so the UI can offer a retry.
 *   3. `startListening()` after an error actually RE-ATTEMPTS (surfacing the
 *      failure again) instead of silently no-opping forever.
 *   4. State listeners observe the recovery transition back to `idle`.
 *
 * The native speech module is unavailable in the jest runtime
 * (`NativeModules.PPTSpeech` resolves to `null`), which is exactly the failure
 * path we need — no extra mocking required. Haptics are disabled so the test
 * doesn't depend on the native `Vibration` module.
 */
import { VoiceAssistant } from './VoiceAssistant';

describe('VoiceAssistant error recovery (no dead-end)', () => {
  it('lands in the error state when the native speech module is unavailable', async () => {
    const assistant = new VoiceAssistant({ hapticFeedback: false });
    await expect(assistant.startListening()).rejects.toThrow();
    expect(assistant.getState()).toBe('error');
  });

  it('reset() recovers from the error state back to idle', async () => {
    const assistant = new VoiceAssistant({ hapticFeedback: false });
    await expect(assistant.startListening()).rejects.toThrow();
    expect(assistant.getState()).toBe('error');

    // Before the fix there was no public way out of 'error'. reset() must recover.
    await assistant.reset();
    expect(assistant.getState()).toBe('idle');
  });

  it('startListening re-attempts after an error instead of dead-ending', async () => {
    const assistant = new VoiceAssistant({ hapticFeedback: false });
    await expect(assistant.startListening()).rejects.toThrow();
    expect(assistant.getState()).toBe('error');

    // The idle-only guard used to swallow this retry (resolve, no-op). A retry
    // from the error state must actually run again — here it surfaces the same
    // native-unavailable failure rather than leaving the user stuck.
    await expect(assistant.startListening()).rejects.toThrow();
  });

  it('notifies state listeners when recovering from error', async () => {
    const assistant = new VoiceAssistant({ hapticFeedback: false });
    const states: string[] = [];
    assistant.onStateChange((s) => states.push(s));

    await expect(assistant.startListening()).rejects.toThrow();
    await assistant.reset();

    expect(states).toContain('error');
    expect(states[states.length - 1]).toBe('idle');
  });

  it('reset() is a no-op when already idle', async () => {
    const assistant = new VoiceAssistant({ hapticFeedback: false });
    const states: string[] = [];
    assistant.onStateChange((s) => states.push(s));

    await assistant.reset();

    expect(assistant.getState()).toBe('idle');
    // No spurious transition emitted for an already-idle controller.
    expect(states).toEqual([]);
  });
});
