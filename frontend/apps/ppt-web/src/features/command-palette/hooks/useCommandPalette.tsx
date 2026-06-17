/**
 * Command Palette Hook and Context
 * Epic 129: Command Palette
 */

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { isLocalStorageAvailable } from '../../../lib/storage';
import type { Command, CommandPaletteContextValue } from '../types';

const RECENT_COMMANDS_KEY = 'ppt-command-palette-recent';
const MAX_RECENT_COMMANDS = 5;

const CommandPaletteContext = createContext<CommandPaletteContextValue | null>(null);

export function CommandPaletteProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);
  const [commands, setCommands] = useState<Command[]>([]);
  const [recentCommandIds, setRecentCommandIds] = useState<string[]>(() => {
    if (!isLocalStorageAvailable()) return [];
    try {
      const stored = localStorage.getItem(RECENT_COMMANDS_KEY);
      return stored ? JSON.parse(stored) : [];
    } catch {
      return [];
    }
  });

  // Register keyboard shortcut (Cmd/Ctrl + K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setIsOpen((prev) => !prev);
      }
      if (e.key === 'Escape' && isOpen) {
        e.preventDefault();
        setIsOpen(false);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen]);

  // Persist recent commands
  useEffect(() => {
    if (isLocalStorageAvailable()) {
      try {
        localStorage.setItem(RECENT_COMMANDS_KEY, JSON.stringify(recentCommandIds));
      } catch {
        // Ignore storage errors
      }
    }
  }, [recentCommandIds]);

  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);
  const toggle = useCallback(() => setIsOpen((prev) => !prev), []);

  const registerCommand = useCallback((command: Command) => {
    setCommands((prev) => {
      const existing = prev.find((c) => c.id === command.id);
      if (existing) {
        return prev.map((c) => (c.id === command.id ? command : c));
      }
      return [...prev, command];
    });
  }, []);

  const unregisterCommand = useCallback((commandId: string) => {
    setCommands((prev) => prev.filter((c) => c.id !== commandId));
  }, []);

  const addToRecent = useCallback((commandId: string) => {
    setRecentCommandIds((prev) => {
      const filtered = prev.filter((id) => id !== commandId);
      return [commandId, ...filtered].slice(0, MAX_RECENT_COMMANDS);
    });
  }, []);

  const recentCommands = useMemo(() => {
    return recentCommandIds
      .map((id) => commands.find((c) => c.id === id))
      .filter((c): c is Command => c !== undefined && !c.hidden);
  }, [recentCommandIds, commands]);

  const value: CommandPaletteContextValue = useMemo(
    () => ({
      isOpen,
      open,
      close,
      toggle,
      registerCommand,
      unregisterCommand,
      commands: commands.filter((c) => !c.hidden),
      recentCommands,
      addToRecent,
    }),
    [
      isOpen,
      open,
      close,
      toggle,
      registerCommand,
      unregisterCommand,
      commands,
      recentCommands,
      addToRecent,
    ]
  );

  return <CommandPaletteContext.Provider value={value}>{children}</CommandPaletteContext.Provider>;
}

export function useCommandPalette(): CommandPaletteContextValue {
  const context = useContext(CommandPaletteContext);
  if (!context) {
    throw new Error('useCommandPalette must be used within CommandPaletteProvider');
  }
  return context;
}

/** Hook to register a command on mount and unregister on unmount */
export function useRegisterCommand(command: Command | null) {
  const { registerCommand, unregisterCommand } = useCommandPalette();

  useEffect(() => {
    if (command) {
      registerCommand(command);
      return () => unregisterCommand(command.id);
    }
  }, [command, registerCommand, unregisterCommand]);
}

/** Hook to register multiple commands */
export function useRegisterCommands(commands: Command[]) {
  const { registerCommand, unregisterCommand } = useCommandPalette();

  useEffect(() => {
    for (const command of commands) {
      registerCommand(command);
    }
    return () => {
      for (const command of commands) {
        unregisterCommand(command.id);
      }
    };
  }, [commands, registerCommand, unregisterCommand]);
}
