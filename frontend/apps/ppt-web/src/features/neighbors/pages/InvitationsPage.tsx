/**
 * InvitationsPage Component
 *
 * Page for viewing and managing neighbor invitations.
 * Presentational component - receives data as props.
 */

import type React from 'react';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { Invitation, InvitationStatus } from '../types';

export interface InvitationsPageProps {
  invitations: Invitation[];
  isLoading?: boolean;
  error?: string | null;
  onResend?: (invitation: Invitation) => void;
  onCancel?: (invitation: Invitation) => void;
  onInviteNew?: () => void;
  onBack?: () => void;
}

type TabType = 'pending' | 'sent' | 'expired';

export const InvitationsPage: React.FC<InvitationsPageProps> = ({
  invitations,
  isLoading = false,
  error = null,
  onResend,
  onCancel,
  onInviteNew,
  onBack,
}) => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>('pending');

  // Filter invitations by tab
  const filteredInvitations = useMemo(() => {
    return invitations.filter((invitation) => {
      switch (activeTab) {
        case 'pending':
          return invitation.status === 'pending';
        case 'sent':
          return invitation.status === 'sent';
        case 'expired':
          return invitation.status === 'expired' || invitation.status === 'cancelled';
        default:
          return true;
      }
    });
  }, [invitations, activeTab]);

  // Count invitations by status
  const counts = useMemo(() => {
    return invitations.reduce(
      (acc, inv) => {
        if (inv.status === 'pending') acc.pending++;
        else if (inv.status === 'sent') acc.sent++;
        else if (inv.status === 'expired' || inv.status === 'cancelled') acc.expired++;
        return acc;
      },
      { pending: 0, sent: 0, expired: 0 }
    );
  }, [invitations]);

  const handleTabChange = useCallback((tab: TabType) => {
    setActiveTab(tab);
  }, []);

  const formatDate = useCallback((dateString: string) => {
    return new Date(dateString).toLocaleDateString(getActiveLocale(), {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }, []);

  const getStatusBadgeClass = useCallback((status: InvitationStatus) => {
    switch (status) {
      case 'pending':
        return 'badge-warning';
      case 'sent':
        return 'badge-info';
      case 'accepted':
        return 'badge-success';
      case 'expired':
        return 'badge-danger';
      case 'cancelled':
        return 'badge-secondary';
      default:
        return '';
    }
  }, []);

  const isExpiringSoon = useCallback((expiresAt: string) => {
    const expiryDate = new Date(expiresAt);
    const now = new Date();
    const daysUntilExpiry = Math.ceil(
      (expiryDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)
    );
    return daysUntilExpiry <= 3 && daysUntilExpiry > 0;
  }, []);

  if (isLoading) {
    return (
      <div className="invitations-page">
        <div className="invitations-page-loading">
          <div className="invitations-page-spinner" />
          <p>{t('neighbors.invitation.loading')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="invitations-page">
      <div className="invitations-page-header">
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            className="invitations-page-back"
            aria-label={t('common.back')}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="invitations-page-back-icon"
            >
              <path d="M19 12H5M12 19l-7-7 7-7" />
            </svg>
            {t('neighbors.backToNeighbors')}
          </button>
        )}
        <div className="invitations-page-title-section">
          <h1>{t('neighbors.invitation.listTitle')}</h1>
          <p className="invitations-page-description">
            {t('neighbors.invitation.listDescription')}
          </p>
        </div>
        {onInviteNew && (
          <button type="button" onClick={onInviteNew} className="invitations-page-invite-button">
            {t('neighbors.invitation.inviteNew')}
          </button>
        )}
      </div>

      {error && (
        <div className="invitations-page-error" role="alert">
          <p>{error}</p>
        </div>
      )}

      <div className="invitations-page-tabs">
        <button
          type="button"
          className={`invitations-page-tab ${activeTab === 'pending' ? 'active' : ''}`}
          onClick={() => handleTabChange('pending')}
        >
          {t('neighbors.invitation.tabPending')}
          {counts.pending > 0 && (
            <span className="invitations-page-tab-count">{counts.pending}</span>
          )}
        </button>
        <button
          type="button"
          className={`invitations-page-tab ${activeTab === 'sent' ? 'active' : ''}`}
          onClick={() => handleTabChange('sent')}
        >
          {t('neighbors.invitation.tabSent')}
          {counts.sent > 0 && <span className="invitations-page-tab-count">{counts.sent}</span>}
        </button>
        <button
          type="button"
          className={`invitations-page-tab ${activeTab === 'expired' ? 'active' : ''}`}
          onClick={() => handleTabChange('expired')}
        >
          {t('neighbors.invitation.tabExpired')}
          {counts.expired > 0 && (
            <span className="invitations-page-tab-count">{counts.expired}</span>
          )}
        </button>
      </div>

      <div className="invitations-page-content">
        {filteredInvitations.length === 0 ? (
          <div className="invitations-page-empty">
            <div className="invitations-page-empty-icon">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
              </svg>
            </div>
            <h2>{t('neighbors.invitation.noInvitations')}</h2>
            <p>
              {t(
                `neighbors.invitation.noInvitations${activeTab.charAt(0).toUpperCase() + activeTab.slice(1)}Description`
              )}
            </p>
            {onInviteNew && activeTab !== 'expired' && (
              <button type="button" onClick={onInviteNew} className="invitations-page-empty-button">
                {t('neighbors.invitation.inviteNew')}
              </button>
            )}
          </div>
        ) : (
          <div className="invitations-page-list">
            {filteredInvitations.map((invitation) => (
              <div key={invitation.id} className="invitations-page-card">
                <div className="invitations-page-card-header">
                  <div className="invitations-page-card-info">
                    <h3 className="invitations-page-card-email">{invitation.email}</h3>
                    {(invitation.firstName || invitation.lastName) && (
                      <p className="invitations-page-card-name">
                        {[invitation.firstName, invitation.lastName].filter(Boolean).join(' ')}
                      </p>
                    )}
                  </div>
                  <span
                    className={`invitations-page-card-badge ${getStatusBadgeClass(invitation.status)}`}
                  >
                    {t(
                      `neighbors.invitation.status${invitation.status.charAt(0).toUpperCase() + invitation.status.slice(1)}`
                    )}
                  </span>
                </div>

                <div className="invitations-page-card-details">
                  <div className="invitations-page-card-detail">
                    <span className="invitations-page-card-label">
                      {t('neighbors.invitation.unit')}:
                    </span>
                    <span className="invitations-page-card-value">
                      {invitation.unitNumber || invitation.unitId}
                    </span>
                  </div>
                  <div className="invitations-page-card-detail">
                    <span className="invitations-page-card-label">
                      {t('neighbors.invitation.role')}:
                    </span>
                    <span className="invitations-page-card-value">
                      {t(
                        `neighbors.invitation.role${invitation.role.charAt(0).toUpperCase() + invitation.role.slice(1).replace('_', '')}`
                      )}
                    </span>
                  </div>
                  <div className="invitations-page-card-detail">
                    <span className="invitations-page-card-label">
                      {t('neighbors.invitation.sentAt')}:
                    </span>
                    <span className="invitations-page-card-value">
                      {formatDate(invitation.sentAt)}
                    </span>
                  </div>
                  <div className="invitations-page-card-detail">
                    <span className="invitations-page-card-label">
                      {t('neighbors.invitation.expiresAt')}:
                    </span>
                    <span
                      className={`invitations-page-card-value ${
                        isExpiringSoon(invitation.expiresAt) ? 'expiring-soon' : ''
                      }`}
                    >
                      {formatDate(invitation.expiresAt)}
                      {isExpiringSoon(invitation.expiresAt) && (
                        <span className="expiring-warning">
                          {t('neighbors.invitation.expiringSoon')}
                        </span>
                      )}
                    </span>
                  </div>
                </div>

                {invitation.personalMessage && (
                  <div className="invitations-page-card-message">
                    <span className="invitations-page-card-label">
                      {t('neighbors.invitation.personalMessage')}:
                    </span>
                    <p className="invitations-page-card-message-text">
                      {invitation.personalMessage}
                    </p>
                  </div>
                )}

                {(invitation.status === 'pending' || invitation.status === 'sent') && (
                  <div className="invitations-page-card-actions">
                    {onResend && (
                      <button
                        type="button"
                        onClick={() => onResend(invitation)}
                        className="invitations-page-card-button secondary"
                      >
                        {t('neighbors.invitation.resend')}
                      </button>
                    )}
                    {onCancel && (
                      <button
                        type="button"
                        onClick={() => onCancel(invitation)}
                        className="invitations-page-card-button danger"
                      >
                        {t('neighbors.invitation.cancel')}
                      </button>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

InvitationsPage.displayName = 'InvitationsPage';
