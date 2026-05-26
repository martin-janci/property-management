/**
 * useContextualHelp — tracks whether the help sidebar is open and which
 * article to show, based on the current router location.
 *
 * The AdminLayout calls this hook and passes the returned props down to
 * the help button and HelpSidebar.
 */

import { useCallback, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { type HelpArticle, getArticleForPath } from '../../help/articles';

export interface ContextualHelpState {
  /** True when the help sidebar is visible. */
  isOpen: boolean;
  /** The article matched to the current page. */
  article: HelpArticle;
  /** Open the help sidebar. */
  open: () => void;
  /** Close the help sidebar. */
  close: () => void;
  /** Toggle the help sidebar. */
  toggle: () => void;
}

export function useContextualHelp(): ContextualHelpState {
  const { pathname } = useLocation();
  const [isOpen, setIsOpen] = useState(false);

  const article = getArticleForPath(pathname);

  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);
  const toggle = useCallback(() => setIsOpen((prev) => !prev), []);

  return { isOpen, article, open, close, toggle };
}
