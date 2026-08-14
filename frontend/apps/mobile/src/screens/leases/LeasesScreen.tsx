/**
 * LeasesScreen (UC-34.1).
 *
 * Lists active and historical leases for the user's organization.
 *
 * Wired to `GET /api/v1/leases?organization_id=<tenant>` on the api-server.
 * That route runs under `RlsConnection` and requires the `organization_id`
 * query param to equal the authenticated tenant — `useApiQuery` sends the
 * bearer token + `X-Tenant-ID`, and the org id comes from `useTenantId()`
 * (the JWT `tenant_id` claim).
 */

import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery, useTenantId } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type LeaseStatus = 'active' | 'pending' | 'expired' | 'terminated';
export type LeaseRole = 'tenant' | 'landlord';

export interface Lease {
  id: string;
  unitLabel: string;
  buildingName: string;
  role: LeaseRole;
  rentAmount: number;
  currency: string;
  startsAt: string;
  endsAt: string;
  status: LeaseStatus;
  counterpartyName: string;
}

/** Subset of `LeaseSummary` from `GET /api/v1/leases`. */
export interface ApiLeaseSummary {
  id: string;
  unit_name: string;
  building_name: string;
  tenant_name: string;
  start_date: string;
  end_date: string;
  /** Decimal is serialised as a JSON string by the api-server. */
  monthly_rent: string | number;
  status: string;
  days_until_expiry?: number | null;
}

interface ApiLeasesListResponse {
  items: ApiLeaseSummary[];
  total?: number;
}

/** Narrow an unknown value to a well-formed `ApiLeaseSummary`. The list
 *  endpoint is consumed without a generated client, so the response is
 *  `unknown` at runtime; require the fields the UI dereferences and drop
 *  anything malformed so one bad row can't crash the screen. */
function isApiLeaseSummary(value: unknown): value is ApiLeaseSummary {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === 'string' &&
    typeof v.unit_name === 'string' &&
    typeof v.building_name === 'string' &&
    typeof v.status === 'string'
  );
}

/** Normalise the `/api/v1/leases` response into a validated summary list.
 *  Accepts the `{ items: [...] }` envelope (and a bare array, defensively)
 *  and tolerates any other/garbage shape by returning an empty list. */
export function parseLeaseSummaries(data: unknown): ApiLeaseSummary[] {
  const candidates: unknown[] =
    typeof data === 'object' && data !== null && Array.isArray((data as { items?: unknown }).items)
      ? (data as { items: unknown[] }).items
      : Array.isArray(data)
        ? data
        : [];
  return candidates.filter(isApiLeaseSummary);
}

/** Map the api-server's lease status string onto the UI's narrowed enum.
 *  Exported so the mapping can be unit-tested without rendering the screen. */
export function toUiLeaseStatus(status: string): LeaseStatus {
  switch (status) {
    case 'active':
      return 'active';
    case 'pending':
    case 'draft':
    case 'pending_signature':
      return 'pending';
    case 'terminated':
    case 'cancelled':
      return 'terminated';
    case 'expired':
    case 'ended':
      return 'expired';
    default:
      return 'pending';
  }
}

/** Map an api-server lease summary onto the UI shape used by the screen.
 *  `LeaseSummary` carries no role/currency, so role defaults to `tenant`
 *  (the resident-app perspective) and currency to EUR. Exported for unit
 *  testing without rendering the screen. */
export function toUiLease(l: ApiLeaseSummary): Lease {
  const rent =
    typeof l.monthly_rent === 'number' ? l.monthly_rent : Number.parseFloat(l.monthly_rent ?? '');
  return {
    id: l.id,
    unitLabel: l.unit_name,
    buildingName: l.building_name,
    role: 'tenant',
    rentAmount: Number.isFinite(rent) ? rent : 0,
    currency: 'EUR',
    startsAt: l.start_date,
    endsAt: l.end_date,
    status: toUiLeaseStatus(l.status),
    counterpartyName: l.tenant_name,
  };
}

function statusColor(status: LeaseStatus): { background: string; color: string } {
  switch (status) {
    case 'active':
      return { background: colors.successBg, color: colors.successDark };
    case 'pending':
      return { background: colors.warningBg, color: colors.warningDark };
    case 'expired':
      return { background: colors.border, color: colors.textSecondary };
    case 'terminated':
      return { background: colors.dangerBg, color: colors.danger };
  }
}

interface LeasesScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function LeasesScreen({ onNavigate }: LeasesScreenProps) {
  const { t } = useTranslation();
  const { tenantId } = useTenantId();

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiLeasesListResponse>(
    ['leases', 'list', tenantId],
    `/api/v1/leases?organization_id=${tenantId ?? ''}`,
    { staleTime: 30_000, enabled: !!tenantId }
  );

  const leases: Lease[] = parseLeaseSummaries(data).map(toUiLease);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Leases</Text>
        <Text style={s.headerSubtitle}>Your active and past rental agreements.</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading || !tenantId ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('leases.loadError')}</Text>
          </View>
        ) : leases.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📑</Text>
            <Text style={s.emptyTitle}>{t('leases.emptyTitle')}</Text>
            <Text style={s.emptyText}>{t('leases.emptyText')}</Text>
          </View>
        ) : (
          leases.map((lease) => {
            const sc = statusColor(lease.status);
            return (
              <Pressable
                key={lease.id}
                style={s.card}
                onPress={() => onNavigate?.('LeaseDetail', { leaseId: lease.id })}
              >
                <View style={s.cardHeader}>
                  <Text style={s.cardTitle}>
                    {lease.unitLabel} · {lease.buildingName}
                  </Text>
                  <View style={[s.badge, { backgroundColor: sc.background }]}>
                    <Text style={[s.badgeText, { color: sc.color }]}>{lease.status}</Text>
                  </View>
                </View>
                <Text style={s.cardBody}>
                  As {lease.role}: {lease.counterpartyName}
                </Text>
                <Text style={[s.cardBody, { fontWeight: '600' }]}>
                  {lease.rentAmount} {lease.currency} / month
                </Text>
                <Text style={[s.cardMeta, { marginTop: 8 }]}>
                  {formatDate(lease.startsAt)} – {formatDate(lease.endsAt)}
                </Text>
              </Pressable>
            );
          })
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

// Reserved for future overrides.
export const LEASES_SCREEN_STYLES = StyleSheet.create({});
