/**
 * FormsScreen (UC-09).
 *
 * Lists forms residents need to complete (consent, declarations, surveys).
 *
 * Wired to `GET /api/v1/forms/available` on the api-server, which returns an
 * `AvailableFormsResponse { forms: FormSummary[] }` (see
 * `db::models::form::FormSummary` and `routes::forms::types` — default
 * snake_case serde). `list_available_forms` runs under `RlsConnection`, so the
 * bearer token + `X-Tenant-ID` header that `useApiQuery` sends are sufficient
 * (unlike `GET /api/v1/forms`, which is manager-only). The response is consumed
 * without a generated client, so it is validated defensively.
 *
 * `FormSummary` carries the form's own lifecycle status, not a per-resident
 * completion state, so the screen presents available forms as actionable
 * ("pending") and surfaces the signature requirement + submission deadline.
 */

import { useCallback } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type FormStatus = 'pending' | 'in_progress' | 'completed';

export interface ResidentForm {
  id: string;
  title: string;
  description: string;
  dueAt?: string;
  status: FormStatus;
  required: boolean;
}

/** Subset of `FormSummary` from `GET /api/v1/forms/available`. */
export interface ApiFormSummary {
  id: string;
  title: string;
  description?: string | null;
  category?: string | null;
  status?: string | null;
  require_signatures?: boolean;
  submission_deadline?: string | null;
}

interface ApiAvailableFormsResponse {
  forms: ApiFormSummary[];
}

function isApiFormSummary(value: unknown): value is ApiFormSummary {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.id === 'string' && typeof v.title === 'string';
}

/** Normalise the `/api/v1/forms/available` response into a validated list.
 *  Accepts the `{ forms: [...] }` envelope and a bare array (defensively);
 *  any other shape yields `[]`. Malformed rows are dropped. */
export function parseFormSummaries(data: unknown): ApiFormSummary[] {
  const candidates: unknown[] =
    typeof data === 'object' &&
    data !== null &&
    Array.isArray((data as ApiAvailableFormsResponse).forms)
      ? (data as ApiAvailableFormsResponse).forms
      : Array.isArray(data)
        ? data
        : [];
  return candidates.filter(isApiFormSummary);
}

/** Map an api-server form summary onto the UI shape. Exported for unit
 *  testing without rendering the screen. */
export function toResidentForm(f: ApiFormSummary): ResidentForm {
  return {
    id: f.id,
    title: f.title,
    description: f.description?.trim() || '',
    dueAt: f.submission_deadline ?? undefined,
    status: 'pending',
    required: f.require_signatures ?? false,
  };
}

function statusBadgeStyle(status: FormStatus) {
  switch (status) {
    case 'pending':
      return { background: colors.dangerBg, color: colors.danger };
    case 'in_progress':
      return { background: colors.warningBg, color: colors.warningDark };
    case 'completed':
      return { background: colors.successBg, color: colors.successDark };
  }
}

interface FormsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function FormsScreen({ onNavigate }: FormsScreenProps) {
  const { data, isLoading, error, refetch, isFetching } = useApiQuery<unknown>(
    ['forms', 'available'],
    '/api/v1/forms/available',
    { staleTime: 30_000 }
  );

  const forms: ResidentForm[] = parseFormSummaries(data).map(toResidentForm);
  const pending = forms.filter((f) => f.status !== 'completed').length;

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Forms</Text>
        {pending > 0 && (
          <Text style={s.headerSubtitle}>
            {pending} form{pending === 1 ? '' : 's'} need{pending === 1 ? 's' : ''} attention
          </Text>
        )}
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>Loading…</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>Couldn't load forms</Text>
            <Text style={s.emptyText}>{error.message}</Text>
          </View>
        ) : forms.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📝</Text>
            <Text style={s.emptyTitle}>No forms</Text>
            <Text style={s.emptyText}>You have no forms to complete right now.</Text>
          </View>
        ) : (
          forms.map((form) => {
            const style = statusBadgeStyle(form.status);
            return (
              <Pressable
                key={form.id}
                style={s.card}
                onPress={() => onNavigate?.('FormFill', { formId: form.id })}
              >
                <View style={s.cardHeader}>
                  <Text style={s.cardTitle} numberOfLines={2}>
                    {form.title}
                  </Text>
                  <View style={[s.badge, { backgroundColor: style.background }]}>
                    <Text style={[s.badgeText, { color: style.color }]}>
                      {form.status.replace('_', ' ')}
                    </Text>
                  </View>
                </View>
                {form.description ? <Text style={s.cardBody}>{form.description}</Text> : null}
                <View style={s.cardFooter}>
                  {form.required && (
                    <View style={[s.badge, { backgroundColor: colors.dangerBg }]}>
                      <Text style={[s.badgeText, { color: colors.danger }]}>Required</Text>
                    </View>
                  )}
                  {form.dueAt && (
                    <Text style={[s.cardMeta, { marginLeft: 'auto' }]}>
                      Due {new Date(form.dueAt).toLocaleDateString()}
                    </Text>
                  )}
                </View>
              </Pressable>
            );
          })
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

// Reserved for future use.
export const FORMS_SCREEN_STYLES = StyleSheet.create({});
