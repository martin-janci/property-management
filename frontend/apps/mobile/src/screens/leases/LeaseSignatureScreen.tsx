/**
 * LeaseSignatureScreen (gap-84-2 / e-signature mobile flow).
 *
 * Shows a pending e-signature request for a lease document and lets the
 * tenant accept or decline. Actions wire to POST /api/v1/esignatures/{id}/sign.
 *
 * Webhook status is HMAC-verified server-side (the `X-PPT-Signature` header
 * on the inbound webhook contains `HMAC-SHA256(secret, body)`); the mobile
 * app only polls the GET endpoint and displays the current status returned
 * by the server — it never trusts client-side state for the signed/declined
 * outcome.
 *
 * API shape:
 *   GET  /api/v1/esignatures/{id}              → ESignatureRequest
 *   POST /api/v1/esignatures/{id}/sign         → ESignatureResult
 *        body: { action: 'sign' | 'decline', signer_name: string }
 */

import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useApiMutation, useApiQuery } from '../../hooks/useApi';
import { formatDateTime } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

// ─── API Types ─────────────────────────────────────────────────────────────────

export type ESignatureStatus = 'pending' | 'signed' | 'declined' | 'expired' | 'cancelled';

/** Mirrors `ESignatureRequest` from the api-server. */
export interface ESignatureRequest {
  id: string;
  lease_id: string;
  document_title: string;
  document_description: string | null;
  requester_name: string;
  signer_name: string | null;
  status: ESignatureStatus;
  requested_at: string;
  expires_at: string | null;
  signed_at: string | null;
  /** HMAC-verified webhook delivery status from the server. */
  webhook_delivered: boolean;
}

interface SignActionVariables {
  action: 'sign' | 'decline';
  signer_name: string;
}

/** Server response after sign/decline action. */
interface ESignatureResult {
  id: string;
  status: ESignatureStatus;
  signed_at: string | null;
  signer_name: string | null;
}

// ─── Screen ────────────────────────────────────────────────────────────────────

interface LeaseSignatureScreenProps {
  /** The e-signature request ID from navigation params. */
  esignatureId?: string;
  onBack?: () => void;
}

export function LeaseSignatureScreen({ esignatureId, onBack }: LeaseSignatureScreenProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [signerName, setSignerName] = useState('');

  // All hooks must run unconditionally on every render (Rules of Hooks), so
  // they are called before the missing-id guard below. The query is disabled
  // until an id is present, so no request fires for the missing-id case.
  const requestQuery = useApiQuery<ESignatureRequest>(
    ['esignatures', 'detail', esignatureId ?? '__missing__'],
    `/api/v1/esignatures/${esignatureId ?? ''}`,
    { staleTime: 15_000, refetchInterval: 30_000, enabled: Boolean(esignatureId) }
  );

  const signMutation = useApiMutation<ESignatureResult, SignActionVariables>(
    `/api/v1/esignatures/${esignatureId ?? ''}/sign`,
    'POST'
  );

  // Guard against missing navigation param (after all hook calls).
  if (!esignatureId) {
    return (
      <View style={s.container}>
        <View style={s.header}>
          {onBack && (
            <Pressable onPress={onBack} style={styles.backLink}>
              <Text style={styles.backLinkText}>← {t('common.back')}</Text>
            </Pressable>
          )}
          <Text style={s.headerTitle}>{t('esignature.title')}</Text>
        </View>
        <View style={s.emptyState}>
          <Text style={s.emptyIcon}>⚠️</Text>
          <Text style={s.emptyTitle}>{t('esignature.missingId')}</Text>
        </View>
      </View>
    );
  }

  const request = requestQuery.data;
  const isLoading = requestQuery.isLoading;
  const loadError = requestQuery.error;

  const isPending = request?.status === 'pending';
  const isActing = signMutation.isPending;

  // Pre-fill signer name from the server-provided value if still empty.
  const effectiveSignerName = signerName || (request?.signer_name ?? '');

  const handleAction = (action: 'sign' | 'decline') => {
    const name = effectiveSignerName.trim();
    if (!name) {
      Alert.alert(t('esignature.signerNameRequired'), t('esignature.signerNameRequiredMessage'));
      return;
    }

    const titleKey =
      action === 'sign' ? 'esignature.confirmSignTitle' : 'esignature.confirmDeclineTitle';
    const messageKey =
      action === 'sign' ? 'esignature.confirmSignMessage' : 'esignature.confirmDeclineMessage';
    const actionLabel = action === 'sign' ? t('esignature.sign') : t('esignature.decline');

    Alert.alert(t(titleKey), t(messageKey), [
      { text: t('common.cancel'), style: 'cancel' },
      {
        text: actionLabel,
        style: action === 'decline' ? 'destructive' : 'default',
        onPress: () => {
          signMutation.mutate(
            { action, signer_name: name },
            {
              onSuccess: () => {
                // Invalidate both this request and the leases list so callers
                // see the updated status immediately.
                queryClient.invalidateQueries({
                  queryKey: ['esignatures', 'detail', esignatureId],
                });
                queryClient.invalidateQueries({ queryKey: ['esignatures', 'list'] });
                queryClient.invalidateQueries({ queryKey: ['leases'] });

                const successKey =
                  action === 'sign' ? 'esignature.signedSuccess' : 'esignature.declinedSuccess';
                Alert.alert(t('esignature.actionComplete'), t(successKey), [
                  { text: t('common.ok'), onPress: () => onBack?.() },
                ]);
              },
              onError: (err) => {
                const errorKey =
                  action === 'sign' ? 'esignature.signFailed' : 'esignature.declineFailed';
                Alert.alert(t(errorKey), err.message);
              },
            }
          );
        },
      },
    ]);
  };

  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← {t('esignature.backToLease')}</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle}>{t('esignature.title')}</Text>
        {request && <Text style={s.headerSubtitle}>{request.document_title}</Text>}
      </View>

      <ScrollView style={s.scrollView}>
        {isLoading ? (
          <View style={s.emptyState}>
            <ActivityIndicator color={colors.accent} />
            <Text style={[s.emptyText, { marginTop: 16 }]}>{t('common.loading')}</Text>
          </View>
        ) : loadError ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('esignature.loadError')}</Text>
            <Pressable
              style={[s.primaryButton, { marginTop: 24 }]}
              onPress={() => requestQuery.refetch()}
            >
              <Text style={s.primaryButtonText}>{t('common.retry')}</Text>
            </Pressable>
          </View>
        ) : request ? (
          <>
            {/* Status banner */}
            <StatusBanner status={request.status} webhookDelivered={request.webhook_delivered} />

            {/* Document info card */}
            <View style={s.card}>
              <Text style={styles.sectionLabel}>{t('esignature.documentLabel')}</Text>
              <Text style={s.cardTitle}>{request.document_title}</Text>
              {request.document_description ? (
                <Text style={[s.cardBody, { marginTop: 6 }]}>{request.document_description}</Text>
              ) : null}
            </View>

            {/* Request metadata card */}
            <View style={s.card}>
              <MetaRow label={t('esignature.requestedBy')} value={request.requester_name} />
              <MetaRow
                label={t('esignature.requestedAt')}
                value={formatDateTime(request.requested_at)}
              />
              {request.expires_at ? (
                <MetaRow
                  label={t('esignature.expiresAt')}
                  value={formatDateTime(request.expires_at)}
                />
              ) : null}
              {request.signed_at ? (
                <MetaRow
                  label={
                    request.status === 'signed'
                      ? t('esignature.signedAt')
                      : t('esignature.declinedAt')
                  }
                  value={formatDateTime(request.signed_at)}
                />
              ) : null}
              {request.webhook_delivered && (
                <MetaRow
                  label={t('esignature.webhookStatus')}
                  value={t('esignature.webhookDelivered')}
                />
              )}
            </View>

            {/* Signer name input + action buttons — only shown when pending */}
            {isPending && (
              <>
                <View style={s.card}>
                  <Text style={styles.sectionLabel}>{t('esignature.signerNameLabel')}</Text>
                  <TextInput
                    style={styles.textInput}
                    value={signerName || (request.signer_name ?? '')}
                    onChangeText={setSignerName}
                    placeholder={t('esignature.signerNamePlaceholder')}
                    placeholderTextColor={colors.textSubtle}
                    autoCapitalize="words"
                    autoCorrect={false}
                    editable={!isActing}
                  />
                  <Text style={styles.signerHint}>{t('esignature.signerNameHint')}</Text>
                </View>

                <View style={styles.actionsRow}>
                  <Pressable
                    style={[styles.declineButton, isActing && styles.buttonDisabled]}
                    onPress={() => handleAction('decline')}
                    disabled={isActing}
                  >
                    {isActing ? (
                      <ActivityIndicator color={colors.danger} />
                    ) : (
                      <Text style={styles.declineButtonText}>{t('esignature.decline')}</Text>
                    )}
                  </Pressable>

                  <Pressable
                    style={[styles.signButton, isActing && styles.buttonDisabled]}
                    onPress={() => handleAction('sign')}
                    disabled={isActing}
                  >
                    {isActing ? (
                      <ActivityIndicator color={colors.white} />
                    ) : (
                      <Text style={styles.signButtonText}>{t('esignature.sign')}</Text>
                    )}
                  </Pressable>
                </View>

                <Text style={styles.legalNote}>{t('esignature.legalNote')}</Text>
              </>
            )}

            <View style={s.bottomSpacer} />
          </>
        ) : null}
      </ScrollView>
    </View>
  );
}

// ─── Sub-components ──────────────────────────────────────────────────────────────────

interface StatusBannerProps {
  status: ESignatureStatus;
  webhookDelivered: boolean;
}

function StatusBanner({ status, webhookDelivered }: StatusBannerProps) {
  const { t } = useTranslation();

  const config: Record<ESignatureStatus, { bg: string; ink: string; icon: string; key: string }> = {
    pending: {
      bg: colors.warningBg,
      ink: colors.warningDark,
      icon: '✍️',
      key: 'esignature.statusPending',
    },
    signed: {
      bg: colors.successBg,
      ink: colors.successDark,
      icon: '✅',
      key: 'esignature.statusSigned',
    },
    declined: {
      bg: colors.dangerBg,
      ink: colors.dangerDark,
      icon: '✗',
      key: 'esignature.statusDeclined',
    },
    expired: {
      bg: colors.border,
      ink: colors.textSecondary,
      icon: '⏰',
      key: 'esignature.statusExpired',
    },
    cancelled: {
      bg: colors.border,
      ink: colors.textSecondary,
      icon: '⊘',
      key: 'esignature.statusCancelled',
    },
  };

  const { bg, ink, icon, key } = config[status] ?? config.pending;

  return (
    <View style={[styles.banner, { backgroundColor: bg }]}>
      <Text style={styles.bannerIcon}>{icon}</Text>
      <View style={{ flex: 1 }}>
        <Text style={[styles.bannerText, { color: ink }]}>{t(key)}</Text>
        {status === 'signed' && webhookDelivered && (
          <Text style={[styles.bannerSub, { color: ink }]}>{t('esignature.webhookConfirmed')}</Text>
        )}
      </View>
    </View>
  );
}

interface MetaRowProps {
  label: string;
  value: string;
}

function MetaRow({ label, value }: MetaRowProps) {
  return (
    <View style={styles.metaRow}>
      <Text style={styles.metaLabel}>{label}</Text>
      <Text style={styles.metaValue}>{value}</Text>
    </View>
  );
}

// ─── Styles ────────────────────────────────────────────────────────────────────

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },

  sectionLabel: {
    fontSize: 11,
    fontWeight: '700',
    color: colors.textSubtle,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 6,
  },

  banner: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    padding: 14,
    borderRadius: 10,
    marginBottom: 12,
    gap: 10,
  },
  bannerIcon: { fontSize: 18, lineHeight: 22 },
  bannerText: { fontSize: 14, fontWeight: '600' },
  bannerSub: { fontSize: 12, marginTop: 2, opacity: 0.8 },

  metaRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: 6,
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderBottomColor: colors.border,
  },
  metaLabel: { fontSize: 13, color: colors.textMuted, flex: 1 },
  metaValue: {
    fontSize: 13,
    color: colors.text,
    fontWeight: '500',
    flexShrink: 1,
    textAlign: 'right',
  },

  textInput: {
    backgroundColor: colors.surfaceMuted,
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
    color: colors.text,
    marginBottom: 6,
  },
  signerHint: { fontSize: 12, color: colors.textMuted, lineHeight: 16 },

  actionsRow: {
    flexDirection: 'row',
    gap: 12,
    marginTop: 8,
    marginBottom: 4,
  },
  signButton: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 14,
    borderRadius: 8,
    backgroundColor: colors.accent,
  },
  signButtonText: { color: colors.white, fontSize: 16, fontWeight: '600' },

  declineButton: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 14,
    borderRadius: 8,
    backgroundColor: colors.dangerBg,
    borderWidth: 1,
    borderColor: colors.danger,
  },
  declineButtonText: { color: colors.danger, fontSize: 16, fontWeight: '600' },

  buttonDisabled: { opacity: 0.6 },

  legalNote: {
    fontSize: 11,
    color: colors.textMuted,
    textAlign: 'center',
    lineHeight: 16,
    marginTop: 8,
    marginHorizontal: 8,
  },
});
