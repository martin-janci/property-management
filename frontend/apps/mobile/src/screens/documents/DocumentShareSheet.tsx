/**
 * DocumentShareSheet (Story 7A.5)
 *
 * A modal bottom-sheet that lets a user create and manage shares for a single
 * document.  Supported share types mirror the backend share_type constants:
 *
 *   • user     — share directly with a specific user (by UUID)
 *   • role     — share with everyone in a named role (e.g. "manager")
 *   • building — share with all residents of the document's building
 *   • link     — generate a public token link; optional password protection
 *
 * The sheet is opened from DocumentDetailScreen via the "Share" button and is
 * also exported so it can be reused from the DocumentsScreen row actions.
 */

import * as Clipboard from 'expo-clipboard';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  Modal,
  Pressable,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  TextInput,
  View,
} from 'react-native';
import { formatDate } from '../../i18n/format';
import { colors } from '../shared/screenStyles';
import {
  type CreateShareRequest,
  isValidUserId,
  SHARE_TYPE,
  type ShareType,
  type ShareWithDocument,
  useCreateShare,
  useDocumentShares,
  useRevokeShare,
} from './documentShare';

// ─── Helper ───────────────────────────────────────────────────────────────────────

function shareTypeLabel(t: ReturnType<typeof useTranslation>['t'], type: ShareType): string {
  switch (type) {
    case SHARE_TYPE.USER:
      return t('documentShare.typeUser');
    case SHARE_TYPE.ROLE:
      return t('documentShare.typeRole');
    case SHARE_TYPE.BUILDING:
      return t('documentShare.typeBuilding');
    case SHARE_TYPE.LINK:
      return t('documentShare.typeLink');
  }
}

function isActive(share: ShareWithDocument): boolean {
  if (share.revoked_at) return false;
  if (share.expires_at && new Date(share.expires_at) < new Date()) return false;
  return true;
}

function formatExpiry(dateStr: string): string {
  return formatDate(dateStr, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

// ─── Create-share form ─────────────────────────────────────────────────────────

interface CreateShareFormProps {
  documentId: string;
  onCreated: () => void;
}

function CreateShareForm({ documentId, onCreated }: CreateShareFormProps) {
  const { t } = useTranslation();
  const [shareType, setShareType] = useState<ShareType>(SHARE_TYPE.LINK);
  const [targetId, setTargetId] = useState('');
  const [targetRole, setTargetRole] = useState('');
  const [usePassword, setUsePassword] = useState(false);
  const [password, setPassword] = useState('');
  const [useExpiry, setUseExpiry] = useState(false);
  const [expiryDays, setExpiryDays] = useState('7');

  const createMutation = useCreateShare(documentId);

  const handleCreate = useCallback(async () => {
    // Client-side validation
    if (shareType === SHARE_TYPE.USER) {
      if (!targetId.trim()) {
        Alert.alert(t('documentShare.errorTitle'), t('documentShare.errorUserIdRequired'));
        return;
      }
      if (!isValidUserId(targetId)) {
        Alert.alert(t('documentShare.errorTitle'), t('documentShare.errorUserIdInvalid'));
        return;
      }
    }
    if (shareType === SHARE_TYPE.ROLE && !targetRole.trim()) {
      Alert.alert(t('documentShare.errorTitle'), t('documentShare.errorRoleRequired'));
      return;
    }
    if (usePassword && password.trim().length < 4) {
      Alert.alert(t('documentShare.errorTitle'), t('documentShare.errorPasswordTooShort'));
      return;
    }

    const req: CreateShareRequest = { share_type: shareType };
    if (shareType === SHARE_TYPE.USER) req.target_id = targetId.trim();
    if (shareType === SHARE_TYPE.ROLE) req.target_role = targetRole.trim();
    if (usePassword && password.trim()) req.password = password.trim();
    if (useExpiry) {
      const days = parseInt(expiryDays, 10);
      if (Number.isNaN(days) || days < 1) {
        Alert.alert(t('documentShare.errorTitle'), t('documentShare.errorInvalidDays'));
        return;
      }
      const expiry = new Date();
      expiry.setDate(expiry.getDate() + days);
      req.expires_at = expiry.toISOString();
    }

    try {
      const result = await createMutation.mutateAsync(req);

      // If a public link was returned, offer to copy it
      if (result.share_url) {
        Alert.alert(t('documentShare.shareCreated'), t('documentShare.linkReady'), [
          { text: t('common.cancel'), style: 'cancel' },
          {
            text: t('documentShare.copyLink'),
            onPress: () => {
              // Build an absolute link using the share token path
              const url = `${result.share_url}`;
              Clipboard.setStringAsync(url).catch(() => {
                /* clipboard permission errors are non-fatal */
              });
            },
          },
        ]);
      } else {
        Alert.alert(t('documentShare.shareCreated'), t('documentShare.shareCreatedMessage'));
      }

      // Reset form fields
      setTargetId('');
      setTargetRole('');
      setPassword('');
      setUsePassword(false);
      setUseExpiry(false);
      onCreated();
    } catch (err) {
      const message = err instanceof Error ? err.message : t('errors.generic');
      Alert.alert(t('documentShare.errorTitle'), message);
    }
  }, [
    shareType,
    targetId,
    targetRole,
    usePassword,
    password,
    useExpiry,
    expiryDays,
    createMutation,
    t,
    onCreated,
  ]);

  return (
    <View style={formStyles.container}>
      <Text style={formStyles.sectionTitle}>{t('documentShare.newShare')}</Text>

      {/* Share type picker */}
      <Text style={formStyles.label}>{t('documentShare.shareType')}</Text>
      <View style={formStyles.typeRow}>
        {(Object.values(SHARE_TYPE) as ShareType[]).map((type) => (
          <Pressable
            key={type}
            style={[formStyles.typeChip, shareType === type && formStyles.typeChipActive]}
            onPress={() => setShareType(type)}
          >
            <Text
              style={[formStyles.typeChipText, shareType === type && formStyles.typeChipTextActive]}
            >
              {shareTypeLabel(t, type)}
            </Text>
          </Pressable>
        ))}
      </View>

      {/* Conditional target inputs */}
      {shareType === SHARE_TYPE.USER && (
        <>
          <Text style={formStyles.label}>{t('documentShare.userId')}</Text>
          <TextInput
            style={formStyles.input}
            placeholder={t('documentShare.userIdPlaceholder')}
            value={targetId}
            onChangeText={setTargetId}
            autoCapitalize="none"
            autoCorrect={false}
          />
        </>
      )}

      {shareType === SHARE_TYPE.ROLE && (
        <>
          <Text style={formStyles.label}>{t('documentShare.roleName')}</Text>
          <TextInput
            style={formStyles.input}
            placeholder={t('documentShare.rolePlaceholder')}
            value={targetRole}
            onChangeText={setTargetRole}
            autoCapitalize="none"
            autoCorrect={false}
          />
        </>
      )}

      {/* Password protection (link shares only) */}
      {shareType === SHARE_TYPE.LINK && (
        <View style={formStyles.toggleRow}>
          <Text style={formStyles.toggleLabel}>{t('documentShare.passwordProtect')}</Text>
          <Switch
            value={usePassword}
            onValueChange={setUsePassword}
            trackColor={{ true: colors.accent }}
            thumbColor={colors.white}
          />
        </View>
      )}

      {shareType === SHARE_TYPE.LINK && usePassword && (
        <TextInput
          style={formStyles.input}
          placeholder={t('documentShare.passwordPlaceholder')}
          value={password}
          onChangeText={setPassword}
          secureTextEntry
          autoCapitalize="none"
          autoCorrect={false}
        />
      )}

      {/* Optional expiry */}
      <View style={formStyles.toggleRow}>
        <Text style={formStyles.toggleLabel}>{t('documentShare.setExpiry')}</Text>
        <Switch
          value={useExpiry}
          onValueChange={setUseExpiry}
          trackColor={{ true: colors.accent }}
          thumbColor={colors.white}
        />
      </View>

      {useExpiry && (
        <View style={formStyles.expiryRow}>
          <TextInput
            style={[formStyles.input, formStyles.expiryInput]}
            placeholder="7"
            value={expiryDays}
            onChangeText={setExpiryDays}
            keyboardType="number-pad"
            maxLength={3}
          />
          <Text style={formStyles.expiryUnit}>{t('documentShare.days')}</Text>
        </View>
      )}

      <Pressable
        style={[
          formStyles.createButton,
          createMutation.isPending && formStyles.createButtonDisabled,
        ]}
        onPress={handleCreate}
        disabled={createMutation.isPending}
      >
        {createMutation.isPending ? (
          <ActivityIndicator size="small" color={colors.white} />
        ) : (
          <Text style={formStyles.createButtonText}>{t('documentShare.createShare')}</Text>
        )}
      </Pressable>
    </View>
  );
}

// ─── Existing-shares list ────────────────────────────────────────────────────

interface ShareListProps {
  documentId: string;
  shares: ShareWithDocument[];
  onRevoked: () => void;
}

function ShareList({ documentId, shares, onRevoked }: ShareListProps) {
  const { t } = useTranslation();
  const { revokeAsync } = useRevokeShare(documentId);
  const [revokingId, setRevokingId] = useState<string | null>(null);

  const handleRevoke = useCallback(
    (shareId: string) => {
      Alert.alert(t('documentShare.revokeTitle'), t('documentShare.revokeConfirm'), [
        { text: t('common.cancel'), style: 'cancel' },
        {
          text: t('documentShare.revokeAction'),
          style: 'destructive',
          onPress: async () => {
            setRevokingId(shareId);
            try {
              await revokeAsync(shareId);
              onRevoked();
            } catch (err) {
              const message = err instanceof Error ? err.message : t('errors.generic');
              Alert.alert(t('documentShare.errorTitle'), message);
            } finally {
              setRevokingId(null);
            }
          },
        },
      ]);
    },
    [revokeAsync, onRevoked, t]
  );

  const activeShares = shares.filter(isActive);

  if (activeShares.length === 0) {
    return (
      <View style={listStyles.empty}>
        <Text style={listStyles.emptyText}>{t('documentShare.noShares')}</Text>
      </View>
    );
  }

  return (
    <View>
      <Text style={listStyles.sectionTitle}>{t('documentShare.activeShares')}</Text>
      {activeShares.map((share) => (
        <View key={share.id} style={listStyles.row}>
          <View style={listStyles.rowLeft}>
            <Text style={listStyles.typeLabel}>{shareTypeLabel(t, share.share_type)}</Text>
            {share.target_role && (
              <Text style={listStyles.meta}>
                {t('documentShare.role')}: {share.target_role}
              </Text>
            )}
            {share.expires_at && (
              <Text style={listStyles.meta}>
                {t('documentShare.expires')}: {formatExpiry(share.expires_at)}
              </Text>
            )}
            {share.share_token && (
              <Pressable
                onPress={() => {
                  Clipboard.setStringAsync(`/documents/shared/${share.share_token}`).catch(() => {
                    /* non-fatal */
                  });
                  Alert.alert(t('documentShare.copied'), t('documentShare.linkCopied'));
                }}
              >
                <Text style={listStyles.copyLink}>{t('documentShare.copyLink')}</Text>
              </Pressable>
            )}
          </View>

          <Pressable
            style={[listStyles.revokeBtn, revokingId === share.id && listStyles.revokeBtnDisabled]}
            onPress={() => handleRevoke(share.id)}
            disabled={revokingId === share.id}
          >
            {revokingId === share.id ? (
              <ActivityIndicator size="small" color={colors.danger} />
            ) : (
              <Text style={listStyles.revokeBtnText}>{t('documentShare.revoke')}</Text>
            )}
          </Pressable>
        </View>
      ))}
    </View>
  );
}

// ─── Main sheet ────────────────────────────────────────────────────────────────

export interface DocumentShareSheetProps {
  documentId: string;
  documentTitle: string;
  visible: boolean;
  onClose: () => void;
}

export function DocumentShareSheet({
  documentId,
  documentTitle,
  visible,
  onClose,
}: DocumentShareSheetProps) {
  const { t } = useTranslation();
  const { data, isLoading, refetch } = useDocumentShares(documentId);

  const handleCreated = useCallback(() => {
    refetch().catch(() => {
      /* refetch errors are non-critical */
    });
  }, [refetch]);

  const handleRevoked = useCallback(() => {
    refetch().catch(() => {
      /* refetch errors are non-critical */
    });
  }, [refetch]);

  return (
    <Modal visible={visible} animationType="slide" transparent onRequestClose={onClose}>
      <View style={sheetStyles.overlay}>
        <Pressable style={sheetStyles.backdrop} onPress={onClose} />

        <View style={sheetStyles.sheet}>
          {/* Handle indicator */}
          <View style={sheetStyles.handle} />

          {/* Header */}
          <View style={sheetStyles.header}>
            <View style={sheetStyles.headerLeft}>
              <Text style={sheetStyles.title}>{t('documentShare.title')}</Text>
              <Text style={sheetStyles.subtitle} numberOfLines={1}>
                {documentTitle}
              </Text>
            </View>
            <Pressable style={sheetStyles.closeBtn} onPress={onClose}>
              <Text style={sheetStyles.closeBtnText}>✕</Text>
            </Pressable>
          </View>

          <ScrollView style={sheetStyles.body} showsVerticalScrollIndicator={false}>
            {/* Create new share form */}
            <CreateShareForm documentId={documentId} onCreated={handleCreated} />

            {/* Existing shares */}
            <View style={sheetStyles.divider} />

            {isLoading ? (
              <ActivityIndicator style={sheetStyles.loader} color={colors.accent} />
            ) : (
              <ShareList
                documentId={documentId}
                shares={data?.shares ?? []}
                onRevoked={handleRevoked}
              />
            )}

            <View style={sheetStyles.bottomSpacer} />
          </ScrollView>
        </View>
      </View>
    </Modal>
  );
}

// ─── Styles ────────────────────────────────────────────────────────────────────

const formStyles = StyleSheet.create({
  container: {
    padding: 16,
  },
  sectionTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 12,
  },
  label: {
    fontSize: 13,
    fontWeight: '500',
    color: colors.textSecondary,
    marginBottom: 6,
    marginTop: 12,
  },
  typeRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  typeChip: {
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: colors.surfaceMuted,
    borderWidth: 1,
    borderColor: colors.border,
  },
  typeChipActive: {
    backgroundColor: colors.accent,
    borderColor: colors.accent,
  },
  typeChipText: {
    fontSize: 13,
    color: colors.textMuted,
    fontWeight: '500',
  },
  typeChipTextActive: {
    color: colors.white,
  },
  input: {
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    padding: 12,
    fontSize: 15,
    borderWidth: 1,
    borderColor: colors.border,
    color: colors.text,
  },
  toggleRow: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginTop: 14,
  },
  toggleLabel: {
    fontSize: 14,
    color: colors.text,
  },
  expiryRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 10,
    marginTop: 8,
  },
  expiryInput: {
    width: 80,
  },
  expiryUnit: {
    fontSize: 14,
    color: colors.textMuted,
  },
  createButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    padding: 14,
    alignItems: 'center',
    marginTop: 20,
  },
  createButtonDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  createButtonText: {
    color: colors.white,
    fontSize: 15,
    fontWeight: '600',
  },
});

const listStyles = StyleSheet.create({
  empty: {
    padding: 24,
    alignItems: 'center',
  },
  emptyText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  sectionTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 8,
    paddingHorizontal: 16,
  },
  row: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    paddingVertical: 12,
    paddingHorizontal: 16,
    borderBottomWidth: 1,
    borderBottomColor: colors.surfaceMuted,
  },
  rowLeft: {
    flex: 1,
    marginRight: 12,
  },
  typeLabel: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 2,
  },
  meta: {
    fontSize: 12,
    color: colors.textMuted,
    marginTop: 2,
  },
  copyLink: {
    fontSize: 12,
    color: colors.accent,
    marginTop: 4,
    fontWeight: '500',
  },
  revokeBtn: {
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 6,
    borderWidth: 1,
    borderColor: colors.danger,
    alignItems: 'center',
    justifyContent: 'center',
    minWidth: 70,
  },
  revokeBtnDisabled: {
    borderColor: colors.border,
  },
  revokeBtnText: {
    fontSize: 12,
    color: colors.danger,
    fontWeight: '500',
  },
});

const sheetStyles = StyleSheet.create({
  overlay: {
    flex: 1,
    justifyContent: 'flex-end',
  },
  backdrop: {
    ...StyleSheet.absoluteFill,
    backgroundColor: colors.bgOverlay,
  },
  sheet: {
    backgroundColor: colors.surface,
    borderTopLeftRadius: 20,
    borderTopRightRadius: 20,
    maxHeight: '85%',
    minHeight: '50%',
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: -4 },
    shadowOpacity: 0.08,
    shadowRadius: 12,
    elevation: 12,
  },
  handle: {
    width: 40,
    height: 4,
    backgroundColor: colors.borderStrong,
    borderRadius: 2,
    alignSelf: 'center',
    marginTop: 10,
    marginBottom: 4,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    paddingHorizontal: 20,
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  headerLeft: {
    flex: 1,
    marginRight: 12,
  },
  title: {
    fontSize: 18,
    fontWeight: '700',
    color: colors.text,
  },
  subtitle: {
    fontSize: 13,
    color: colors.textMuted,
    marginTop: 2,
  },
  closeBtn: {
    padding: 4,
  },
  closeBtnText: {
    fontSize: 18,
    color: colors.textMuted,
  },
  body: {
    flex: 1,
  },
  divider: {
    height: 1,
    backgroundColor: colors.border,
    marginVertical: 4,
  },
  loader: {
    marginVertical: 24,
  },
  bottomSpacer: {
    height: 40,
  },
});
