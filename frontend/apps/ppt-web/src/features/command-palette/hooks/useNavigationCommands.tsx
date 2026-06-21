/**
 * Navigation Commands Hook
 * Epic 129: Command Palette
 *
 * Registers default navigation commands for the command palette.
 */

import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import type { Command } from '../types';
import { useRegisterCommands } from './useCommandPalette';

/**
 * Registers navigation commands with the command palette.
 * Should be used within a component rendered inside BrowserRouter.
 */
export function useNavigationCommands() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const commands = useMemo((): Command[] => {
    return [
      // Navigation commands
      {
        id: 'nav-home',
        label: t('commandPalette.commands.goToDashboard'),
        description: t('commandPalette.descriptions.goToDashboard'),
        category: 'navigation',
        keywords: ['home', 'main', 'dashboard', 'start'],
        icon: '🏠',
        action: () => navigate('/'),
      },
      {
        id: 'nav-documents',
        label: t('commandPalette.commands.goToDocuments'),
        description: t('commandPalette.descriptions.goToDocuments'),
        category: 'navigation',
        keywords: ['documents', 'files', 'upload', 'pdf'],
        icon: '📄',
        action: () => navigate('/documents'),
      },
      {
        id: 'nav-news',
        label: t('commandPalette.commands.goToNews'),
        description: t('commandPalette.descriptions.goToNews'),
        category: 'navigation',
        keywords: ['news', 'articles', 'announcements', 'updates'],
        icon: '📰',
        action: () => navigate('/news'),
      },
      {
        id: 'nav-emergency',
        label: t('commandPalette.commands.goToEmergency'),
        description: t('commandPalette.descriptions.goToEmergency'),
        category: 'navigation',
        keywords: ['emergency', 'contacts', 'urgent', 'help', 'phone'],
        icon: '🚨',
        action: () => navigate('/emergency'),
      },
      {
        id: 'nav-disputes',
        label: t('commandPalette.commands.goToDisputes'),
        description: t('commandPalette.descriptions.goToDisputes'),
        category: 'navigation',
        keywords: ['disputes', 'complaints', 'issues', 'mediation'],
        icon: '⚖️',
        action: () => navigate('/disputes'),
      },
      {
        id: 'nav-outages',
        label: t('commandPalette.commands.goToOutages'),
        description: t('commandPalette.descriptions.goToOutages'),
        category: 'navigation',
        keywords: ['outages', 'utilities', 'maintenance', 'water', 'power'],
        icon: '⚡',
        action: () => navigate('/outages'),
      },
      {
        id: 'nav-iot',
        label: t('commandPalette.commands.goToIot', { defaultValue: 'Go to Smart Building' }),
        description: t('commandPalette.descriptions.goToIot', {
          defaultValue: 'IoT sensors, telemetry and alerts',
        }),
        category: 'navigation',
        keywords: ['iot', 'smart building', 'sensors', 'devices', 'telemetry', 'alerts'],
        icon: '📡',
        action: () => navigate('/iot'),
      },
      {
        id: 'nav-iot-sensors',
        label: t('commandPalette.commands.goToSensors', { defaultValue: 'Go to Sensors' }),
        description: t('commandPalette.descriptions.goToSensors', {
          defaultValue: 'Manage registered IoT devices',
        }),
        category: 'navigation',
        keywords: ['sensors', 'devices', 'iot', 'registry'],
        icon: '📡',
        action: () => navigate('/iot/sensors'),
      },
      {
        id: 'nav-iot-alerts',
        label: t('commandPalette.commands.goToIotAlerts', { defaultValue: 'Go to Sensor Alerts' }),
        description: t('commandPalette.descriptions.goToIotAlerts', {
          defaultValue: 'IoT threshold alerts: acknowledge and resolve',
        }),
        category: 'navigation',
        keywords: ['alerts', 'iot', 'sensors', 'threshold', 'acknowledge', 'resolve'],
        icon: '🚨',
        action: () => navigate('/iot/alerts'),
      },
      // Create commands
      {
        id: 'create-dispute',
        label: t('commandPalette.commands.createDispute'),
        description: t('commandPalette.descriptions.createDispute'),
        category: 'create',
        keywords: ['create', 'new', 'file', 'dispute', 'complaint'],
        icon: '➕',
        action: () => navigate('/disputes/new'),
      },
      {
        id: 'create-outage',
        label: t('commandPalette.commands.createOutage'),
        description: t('commandPalette.descriptions.createOutage'),
        category: 'create',
        keywords: ['create', 'new', 'outage', 'schedule', 'maintenance'],
        icon: '➕',
        action: () => navigate('/outages/new'),
      },
      {
        id: 'upload-document',
        label: t('commandPalette.commands.uploadDocument'),
        description: t('commandPalette.descriptions.uploadDocument'),
        category: 'create',
        keywords: ['upload', 'document', 'file', 'pdf', 'add'],
        icon: '📤',
        action: () => navigate('/documents/upload'),
      },
      // Settings commands
      {
        id: 'settings-accessibility',
        label: t('commandPalette.commands.goToAccessibility'),
        description: t('commandPalette.descriptions.goToAccessibility'),
        category: 'settings',
        keywords: ['settings', 'accessibility', 'a11y', 'screen reader', 'contrast'],
        icon: '♿',
        action: () => navigate('/settings/accessibility'),
      },
      {
        id: 'settings-privacy',
        label: t('commandPalette.commands.goToPrivacy'),
        description: t('commandPalette.descriptions.goToPrivacy'),
        category: 'settings',
        keywords: ['settings', 'privacy', 'data', 'consent', 'preferences'],
        icon: '🔒',
        action: () => navigate('/settings/privacy'),
      },
    ];
  }, [t, navigate]);

  useRegisterCommands(commands);
}
