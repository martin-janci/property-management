/**
 * Announcer Component (Epic 125, Story 125.3)
 *
 * Provides screen reader announcements via aria-live regions.
 * Used to announce dynamic content changes, loading states, and errors.
 *
 * @example
 * const { announce } = useAnnouncer();
 * announce('Form submitted successfully');
 */

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react';

interface AnnouncerContextValue {
  /**
   * Announce a message to screen readers.
   * @param message The message to announce
   * @param politeness 'polite' (default) waits for current speech, 'assertive' interrupts
   */
  announce: (message: string, politeness?: 'polite' | 'assertive') => void;
}

const AnnouncerContext = createContext<AnnouncerContextValue | null>(null);

interface AnnouncerProviderProps {
  children: ReactNode;
}

/**
 * Provider component that renders the aria-live regions and provides
 * the announce function via context.
 */
export function AnnouncerProvider({ children }: AnnouncerProviderProps) {
  const [politeMessage, setPoliteMessage] = useState('');
  const [assertiveMessage, setAssertiveMessage] = useState('');

  // Use refs to store timeout IDs for cleanup.
  // We must track BOTH the deferred "set" timeout and the "clear" timeout per
  // channel. If the "set" timeout is left untracked, a rapid second announce()
  // can leave a previous deferred set pending, which then fires after the newer
  // message was already shown — resurrecting a stale screen-reader message.
  const politeSetTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const politeClearTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const assertiveSetTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const assertiveClearTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const announce = useCallback((message: string, politeness: 'polite' | 'assertive' = 'polite') => {
    if (politeness === 'polite') {
      // Cancel any pending set/clear from a prior announce so an in-flight
      // deferred set cannot resurrect a stale message over this one.
      if (politeSetTimeoutRef.current) {
        clearTimeout(politeSetTimeoutRef.current);
      }
      if (politeClearTimeoutRef.current) {
        clearTimeout(politeClearTimeoutRef.current);
      }
      // Clear then set to trigger screen reader.
      setPoliteMessage('');
      politeSetTimeoutRef.current = setTimeout(() => setPoliteMessage(message), 100);

      // Clear message after it's been announced.
      politeClearTimeoutRef.current = setTimeout(() => setPoliteMessage(''), 3000);
    } else {
      if (assertiveSetTimeoutRef.current) {
        clearTimeout(assertiveSetTimeoutRef.current);
      }
      if (assertiveClearTimeoutRef.current) {
        clearTimeout(assertiveClearTimeoutRef.current);
      }
      setAssertiveMessage('');
      assertiveSetTimeoutRef.current = setTimeout(() => setAssertiveMessage(message), 100);

      assertiveClearTimeoutRef.current = setTimeout(() => setAssertiveMessage(''), 3000);
    }
  }, []);

  // Clear any pending timeouts on unmount to avoid setState-after-unmount.
  useEffect(() => {
    return () => {
      if (politeSetTimeoutRef.current) clearTimeout(politeSetTimeoutRef.current);
      if (politeClearTimeoutRef.current) clearTimeout(politeClearTimeoutRef.current);
      if (assertiveSetTimeoutRef.current) clearTimeout(assertiveSetTimeoutRef.current);
      if (assertiveClearTimeoutRef.current) clearTimeout(assertiveClearTimeoutRef.current);
    };
  }, []);

  return (
    <AnnouncerContext.Provider value={{ announce }}>
      {children}
      {/* Polite announcements - waits for current speech */}
      <div role="status" aria-live="polite" aria-atomic="true" className="aria-announcer">
        {politeMessage}
      </div>
      {/* Assertive announcements - interrupts current speech */}
      <div role="alert" aria-live="assertive" aria-atomic="true" className="aria-announcer">
        {assertiveMessage}
      </div>
    </AnnouncerContext.Provider>
  );
}

/**
 * Hook to access the announcer context.
 * @throws Error if used outside AnnouncerProvider
 */
export function useAnnouncer(): AnnouncerContextValue {
  const context = useContext(AnnouncerContext);
  if (!context) {
    throw new Error('useAnnouncer must be used within an AnnouncerProvider');
  }
  return context;
}

AnnouncerProvider.displayName = 'AnnouncerProvider';
