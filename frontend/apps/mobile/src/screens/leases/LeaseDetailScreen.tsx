/**
 * LeaseDetailScreen (UC-34.4).
 *
 * Single-lease view with payment summary and document downloads.
 *
 * Wired to `GET /api/v1/leases/{id}` on the api-server, which returns a
 * `LeaseWithDetails { lease, unit_name, building_name, upcoming_payments, … }`
 * (see `db::models::lease::{Lease, LeasePayment, LeaseWithDetails}` — default
 * snake_case serde, `Decimal`/`NaiveDate` fields as JSON strings). The lease id
 * arrives as a navigation param. The response is consumed without a generated
 * client, so it is validated defensively.
 */

import type {
  Lease as ApiLease,
  LeasePayment as ApiLeasePayment,
  LeaseWithDetails,
} from '@ppt/api-client';
import { useCallback } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { colors, screenStyles as s } from '../shared/screenStyles';
import { type Lease, toUiLeaseStatus } from './LeasesScreen';

export interface LeasePaymentRow {
  id: string;
  period: string;
  amount: number;
  paidAt: string | null;
}

/**
 * The subset of the generated `@ppt/api-client` `LeaseWithDetails` payload
 * (`GET /api/v1/leases/{id}`) that this screen consumes. Deriving it from the
 * generated type (rather than a hand-rolled interface) surfaces OpenAPI drift
 * — a field rename/type-change on `Lease`, `LeasePayment`, or `LeaseWithDetails`
 * breaks compilation here. `ApiLease` / `ApiLeasePayment` are the generated
 * entity types, used by the defensive parser casts below.
 */
export type ApiLeaseDetail = Pick<
  LeaseWithDetails,
  'lease' | 'unit_name' | 'building_name' | 'upcoming_payments'
>;

function toNumber(value: string | number | null | undefined): number {
  if (value == null) return 0;
  const n = typeof value === 'number' ? value : Number.parseFloat(value);
  return Number.isFinite(n) ? n : 0;
}

/** Validate the raw `GET /api/v1/leases/{id}` body, or null on a bad shape. */
export function parseLeaseDetail(data: unknown): ApiLeaseDetail | null {
  if (typeof data !== 'object' || data === null) return null;
  const v = data as Record<string, unknown>;
  const lease = v.lease;
  if (typeof lease !== 'object' || lease === null) return null;
  const l = lease as Record<string, unknown>;
  if (typeof l.id !== 'string') return null;
  return {
    lease: lease as ApiLease,
    unit_name: typeof v.unit_name === 'string' ? v.unit_name : '',
    building_name: typeof v.building_name === 'string' ? v.building_name : '',
    upcoming_payments: Array.isArray(v.upcoming_payments)
      ? (v.upcoming_payments as ApiLeasePayment[])
      : [],
  };
}

/** Map the api-server lease detail onto the UI `Lease` shape. Exported for
 *  unit testing without rendering the screen. */
export function toUiLease(detail: ApiLeaseDetail): Lease {
  const { lease } = detail;
  return {
    id: lease.id,
    unitLabel: detail.unit_name || 'Unit',
    buildingName: detail.building_name || '',
    role: 'tenant',
    rentAmount: toNumber(lease.monthly_rent),
    currency: 'EUR',
    startsAt: lease.start_date,
    endsAt: lease.end_date,
    status: toUiLeaseStatus(lease.status),
    counterpartyName: lease.landlord_name || lease.tenant_name || '',
  };
}

/** Format a payment `due_date` (ISO date) as a "Month Year" period label. */
function periodLabel(dueDate: string): string {
  const d = new Date(dueDate);
  if (Number.isNaN(d.getTime())) return dueDate;
  return d.toLocaleDateString('en-US', { month: 'long', year: 'numeric' });
}

/** Map api-server payments onto the UI payment rows, newest `due_date` first.
 *  Sorting is done on the raw ISO `due_date` (not the formatted month label,
 *  which would sort alphabetically). */
export function toUiPayments(detail: ApiLeaseDetail): LeasePaymentRow[] {
  return (detail.upcoming_payments ?? [])
    .slice()
    .sort((a, b) => new Date(b.due_date).getTime() - new Date(a.due_date).getTime())
    .map((p) => ({
      id: p.id,
      period: periodLabel(p.due_date),
      amount: toNumber(p.amount),
      paidAt: p.paid_at ?? null,
    }));
}

interface LeaseDetailScreenProps {
  leaseId?: string;
  onBack?: () => void;
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function LeaseDetailScreen({ leaseId, onBack, onNavigate }: LeaseDetailScreenProps) {
  const { data, isLoading, error, refetch, isFetching } = useApiQuery<unknown>(
    ['leases', 'detail', leaseId],
    `/api/v1/leases/${leaseId ?? ''}`,
    { staleTime: 30_000, enabled: !!leaseId }
  );

  const detail = parseLeaseDetail(data);
  const lease = detail ? toUiLease(detail) : null;
  const payments = detail ? toUiPayments(detail) : [];

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← Leases</Text>
          </Pressable>
        )}
        {lease ? (
          <>
            <Text style={s.headerTitle}>
              {lease.unitLabel}
              {lease.buildingName ? ` · ${lease.buildingName}` : ''}
            </Text>
            <Text style={s.headerSubtitle}>
              {new Date(lease.startsAt).toLocaleDateString()} –{' '}
              {new Date(lease.endsAt).toLocaleDateString()}
            </Text>
          </>
        ) : (
          <Text style={s.headerTitle}>Lease</Text>
        )}
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading || !leaseId ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>Loading…</Text>
          </View>
        ) : error || !lease ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>Couldn't load lease</Text>
            <Text style={s.emptyText}>{error?.message ?? 'Lease not found.'}</Text>
          </View>
        ) : (
          <>
            <View style={s.card}>
              <Text style={styles.metricLabel}>Monthly rent</Text>
              <Text style={styles.metricValue}>
                {lease.rentAmount} {lease.currency}
              </Text>
              <Text style={[s.cardMeta, { marginTop: 8 }]}>
                As {lease.role}
                {lease.counterpartyName ? `, paid to ${lease.counterpartyName}` : ''}
              </Text>
            </View>

            <Text style={styles.sectionTitle}>Upcoming payments</Text>
            {payments.length === 0 ? (
              <View style={s.card}>
                <Text style={s.cardBody}>No payments recorded yet.</Text>
              </View>
            ) : (
              payments.map((payment) => (
                <View key={payment.id} style={s.card}>
                  <View style={s.cardHeader}>
                    <Text style={s.cardTitle}>{payment.period}</Text>
                    <Text style={s.cardMeta}>
                      {payment.amount} {lease.currency}
                    </Text>
                  </View>
                  <Text style={s.cardBody}>
                    {payment.paidAt
                      ? `Paid on ${new Date(payment.paidAt).toLocaleDateString()}`
                      : 'Awaiting payment'}
                  </Text>
                </View>
              ))
            )}

            <Text style={styles.sectionTitle}>Documents</Text>
            <Pressable
              style={s.card}
              onPress={() =>
                onNavigate?.('DocumentDetail', {
                  documentId: `lease-${lease.id}-contract`,
                })
              }
            >
              <Text style={s.cardTitle}>📄 Lease agreement</Text>
              <Text style={s.cardMeta}>Signed PDF — open to review or share.</Text>
            </Pressable>
          </>
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  metricLabel: {
    fontSize: 12,
    color: colors.textMuted,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  metricValue: { fontSize: 32, fontWeight: '700', color: colors.text, marginTop: 8 },
  sectionTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.textMuted,
    marginTop: 16,
    marginBottom: 8,
    marginHorizontal: 4,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
});
