/**
 * useFocusTrap — minimal focus-trap + Escape handler for modal dialogs.
 *
 * On mount the hook:
 *   - records the currently-focused element so we can restore focus on unmount
 *   - moves focus to the first focusable element inside the container (or the
 *     container itself if none, requiring tabIndex={-1} on the container)
 *
 * While mounted:
 *   - Tab / Shift-Tab cycle within the container (focus trap)
 *   - Escape calls `onEscape` (typically the dialog's cancel handler)
 *
 * On unmount:
 *   - returns focus to the originally-focused element
 *
 * Usage:
 *   const ref = useRef<HTMLDivElement | null>(null);
 *   useFocusTrap(ref, onCancel);
 *   return <div ref={ref} role="dialog" aria-modal="true" tabIndex={-1}>…</div>;
 *
 * This is a deliberately small implementation copied from admin-web's
 * useFocusTrap. It is duplicated here (rather than extracted into a shared
 * package) to keep this a low-risk WCAG 2.1.2 fix; consolidation can happen
 * in a follow-up. It does NOT handle:
 *   - portals (assumes the container is rendered in-place under document.body)
 *   - elements becoming focusable after mount (re-queries on every Tab keydown)
 *   - inert backgrounding of the rest of the page (caller can rely on the
 *     overlay div blocking pointer events)
 */
import { useEffect } from 'react';

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function useFocusTrap(
  containerRef: React.RefObject<HTMLElement | null>,
  onEscape: () => void
): void {
  // biome-ignore lint/correctness/useExhaustiveDependencies: bind once on mount — the dialog lives for a single open cycle; re-binding on onEscape/ref identity would reset focus and lose `previouslyFocused`.
  useEffect(() => {
    const node = containerRef.current;
    if (!node) return;

    const previouslyFocused = document.activeElement as HTMLElement | null;

    // Initial focus: first focusable child, else the container itself.
    const initialFocusable = node.querySelector<HTMLElement>(FOCUSABLE_SELECTOR);
    (initialFocusable ?? node).focus();

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onEscape();
        return;
      }
      if (e.key !== 'Tab') return;

      const focusable = Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
      if (focusable.length === 0) {
        e.preventDefault();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement as HTMLElement | null;

      if (e.shiftKey) {
        if (active === first || !node.contains(active)) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (active === last || !node.contains(active)) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    node.addEventListener('keydown', handleKeyDown);
    return () => {
      node.removeEventListener('keydown', handleKeyDown);
      previouslyFocused?.focus?.();
    };
  }, []);
}
