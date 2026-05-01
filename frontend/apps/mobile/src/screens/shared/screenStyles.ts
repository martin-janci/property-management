/**
 * Shared StyleSheet for the new feature screens (Phase 3).
 *
 * Consolidates the recurring header / list / card / empty-state styles so
 * each new screen stays focused on its content. Mirrors the look of the
 * existing AnnouncementsScreen and FaultsListScreen.
 */

import { StyleSheet } from 'react-native';

export const colors = {
  // ─── Base palette ───────────────────────────────────────────
  background: '#f9fafb',
  surface: '#ffffff',
  surfaceMuted: '#f3f4f6',
  border: '#e5e7eb',
  borderStrong: '#d1d5db',
  divider: '#f3f4f6',
  text: '#111827',
  textSecondary: '#374151',
  textMuted: '#6b7280',
  textSubtle: '#9ca3af',

  // ─── Brand ──────────────────────────────────────────────────
  accent: '#2563eb',
  accentDark: '#1d4ed8',
  accentDisabled: '#93c5fd',
  accentSoft: '#eff6ff',

  // ─── Semantic ───────────────────────────────────────────────
  danger: '#ef4444',
  dangerDark: '#dc2626',
  dangerBg: '#fee2e2',
  warning: '#f59e0b',
  warningDark: '#d97706',
  warningBg: '#fef3c7',
  success: '#10b981',
  successDark: '#047857',
  successBg: '#d1fae5',
  info: '#0ea5e9',
  infoBg: '#e0f2fe',

  // ─── Fault status pill tokens ────────────────────────────────
  statusNewBg: '#fee2e2',
  statusNewInk: '#991b1b',
  statusTriagedBg: '#dbeafe',
  statusTriagedInk: '#1d4ed8',
  statusInProgressBg: '#ede9fe',
  statusInProgressInk: '#5b21b6',
  statusWaitPartsBg: '#ffedd5',
  statusWaitPartsInk: '#9a3412',
  statusScheduledBg: '#eff6ff',
  statusScheduledInk: '#1d4ed8',
  statusResolvedBg: '#d1fae5',
  statusResolvedInk: '#065f46',
  statusClosedBg: '#f3f4f6',
  statusClosedInk: '#374151',
  statusReopenedBg: '#fef3c7',
  statusReopenedInk: '#92400e',

  // ─── Priority ink tokens ─────────────────────────────────────
  priorityLowInk: '#6b7280',
  priorityMediumInk: '#d97706',
  priorityHighInk: '#ea580c',
  priorityUrgentInk: '#dc2626',
  warningInk: '#92400e',
};

export const screenStyles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  header: {
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  headerTitle: {
    fontSize: 24,
    fontWeight: 'bold',
    color: colors.text,
  },
  headerSubtitle: {
    fontSize: 13,
    color: colors.textMuted,
    marginTop: 4,
  },
  searchContainer: {
    padding: 16,
    paddingBottom: 8,
    backgroundColor: colors.surface,
  },
  searchInput: {
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
  },
  filtersWrapper: {
    backgroundColor: colors.surface,
    paddingBottom: 16,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  filtersRow: {
    flexDirection: 'row',
    paddingHorizontal: 16,
    gap: 8,
  },
  filterChip: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: colors.surfaceMuted,
    gap: 4,
  },
  filterChipActive: {
    backgroundColor: colors.accent,
  },
  filterText: {
    fontSize: 14,
    color: colors.textMuted,
    fontWeight: '500',
  },
  filterTextActive: {
    color: '#fff',
  },
  scrollView: {
    flex: 1,
    padding: 16,
  },
  card: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    shadowColor: '#000',
    shadowOpacity: 0.04,
    shadowOffset: { width: 0, height: 1 },
    shadowRadius: 2,
    elevation: 1,
  },
  cardHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: 8,
  },
  cardTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
    flex: 1,
  },
  cardMeta: {
    fontSize: 12,
    color: colors.textMuted,
  },
  cardBody: {
    fontSize: 14,
    color: colors.textMuted,
    marginTop: 4,
  },
  cardFooter: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginTop: 12,
  },
  badge: {
    paddingHorizontal: 8,
    paddingVertical: 2,
    borderRadius: 999,
    backgroundColor: colors.surfaceMuted,
  },
  badgeText: {
    fontSize: 11,
    fontWeight: '600',
    color: colors.textMuted,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  emptyState: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 60,
  },
  emptyIcon: {
    fontSize: 48,
    marginBottom: 16,
  },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 8,
  },
  emptyText: {
    fontSize: 14,
    color: colors.textMuted,
    textAlign: 'center',
    paddingHorizontal: 32,
  },
  primaryButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 14,
    paddingHorizontal: 16,
    borderRadius: 8,
    backgroundColor: colors.accent,
    gap: 8,
    marginTop: 16,
  },
  primaryButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  bottomSpacer: {
    height: 40,
  },
});
