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

import { createContext, type ReactNode, useCallback, useContext, useRef, useState } from 'react';

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

  // Use refs to store timeout IDs for cleanup
  const politeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const assertiveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const announce = useCallback((message: string, politeness: 'polite' | 'assertive' = 'polite') => {
    // Clear existing timeout
    if (politeness === 'polite') {
      if (politeTimeoutRef.current) {
        clearTimeout(politeTimeoutRef.current);
      }
      // Clear then set to trigger screen reader
      setPoliteMessage('');
      setTimeout(() => setPoliteMessage(message), 100);

      // Clear message after it's been announced
      politeTimeoutRef.current = setTimeout(() => setPoliteMessage(''), 3000);
    } else {
      if (assertiveTimeoutRef.current) {
        clearTimeout(assertiveTimeoutRef.current);
      }
      setAssertiveMessage('');
      setTimeout(() => setAssertiveMessage(message), 100);

      assertiveTimeoutRef.current = setTimeout(() => setAssertiveMessage(''), 3000);
    }
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
