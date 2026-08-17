/**
 * RealtorManagement Component
 *
 * Manage realtors within an agency (Epic 45, Story 45.2).
 */

'use client';

import type { Realtor, RealtorStatus } from '@ppt/reality-api-client';
import {
  useInviteRealtor,
  useMyAgency,
  useRealtors,
  useRemoveRealtor,
  useResendInvitation,
  useUpdateRealtor,
} from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import {
  AgencyLoadError,
  isNotFoundError,
  NoAgencyMessage,
  SectionError,
} from './AgencyErrorStates';

type TabType = 'all' | 'active' | 'invited' | 'inactive';

export function RealtorManagement() {
  const t = useTranslations('agency');
  const [activeTab, setActiveTab] = useState<TabType>('all');
  const [showInviteModal, setShowInviteModal] = useState(false);
  const [selectedRealtor, setSelectedRealtor] = useState<Realtor | null>(null);

  const {
    data: agency,
    isLoading: agencyLoading,
    isError: agencyError,
    error: agencyErrorObj,
    refetch: refetchAgency,
  } = useMyAgency();
  const { data: realtors, isLoading, isError: realtorsError } = useRealtors(agency?.id || '');

  const filteredRealtors = realtors?.filter((r) => {
    if (activeTab === 'all') return true;
    if (activeTab === 'active') return r.status === 'active';
    if (activeTab === 'invited') return r.status === 'invited';
    if (activeTab === 'inactive') return r.status === 'inactive' || r.status === 'suspended';
    return true;
  });

  const tabs: { key: TabType; label: string; count: number }[] = [
    { key: 'all', label: t('tabAll'), count: realtors?.length || 0 },
    {
      key: 'active',
      label: t('tabActive'),
      count: realtors?.filter((r) => r.status === 'active').length || 0,
    },
    {
      key: 'invited',
      label: t('tabPending'),
      count: realtors?.filter((r) => r.status === 'invited').length || 0,
    },
    {
      key: 'inactive',
      label: t('tabInactive'),
      count:
        realtors?.filter((r) => r.status === 'inactive' || r.status === 'suspended').length || 0,
    },
  ];

  // Distinguish an agency-load failure from a genuine "no agency" state so a
  // transport/server error no longer renders the misleading "No realtors yet"
  // empty state (Issue #2343, sibling of Issue #2277).
  if (agencyError && !isNotFoundError(agencyErrorObj)) {
    return <AgencyLoadError onRetry={() => refetchAgency()} />;
  }
  if (!agencyLoading && !agency) {
    return <NoAgencyMessage />;
  }

  return (
    <div className="realtor-management">
      {/* Header */}
      <div className="header">
        <div>
          <Link href="/agency" className="back-link">
            ← {t('backToDashboard')}
          </Link>
          <h1 className="title">{t('realtorManagementTitle')}</h1>
          <p className="subtitle">{t('realtorManagementSubtitle')}</p>
        </div>
        <button type="button" className="invite-button" onClick={() => setShowInviteModal(true)}>
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="8.5" cy="7" r="4" />
            <line x1="20" y1="8" x2="20" y2="14" />
            <line x1="23" y1="11" x2="17" y2="11" />
          </svg>
          {t('inviteRealtor')}
        </button>
      </div>

      {/* Tabs */}
      <div className="tabs">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            className={`tab ${activeTab === tab.key ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.key)}
          >
            {tab.label}
            <span className="tab-count">{tab.count}</span>
          </button>
        ))}
      </div>

      {/* Realtor List */}
      <div className="realtor-list">
        {isLoading ? (
          <RealtorListSkeleton />
        ) : realtorsError ? (
          <SectionError message={t('realtorsLoadError')} />
        ) : filteredRealtors?.length === 0 ? (
          <EmptyState tab={activeTab} onInvite={() => setShowInviteModal(true)} />
        ) : (
          filteredRealtors?.map((realtor) => (
            <RealtorCard
              key={realtor.id}
              realtor={realtor}
              agencyId={agency?.id || ''}
              onSelect={() => setSelectedRealtor(realtor)}
            />
          ))
        )}
      </div>

      {/* Invite Modal */}
      {showInviteModal && agency && (
        <InviteRealtorModal agencyId={agency.id} onClose={() => setShowInviteModal(false)} />
      )}

      {/* Realtor Detail Modal */}
      {selectedRealtor && agency && (
        <RealtorDetailModal
          realtor={selectedRealtor}
          agencyId={agency.id}
          onClose={() => setSelectedRealtor(null)}
        />
      )}

      <style jsx>{`
        .realtor-management {
          padding: 24px;
          max-width: 1200px;
          margin: 0 auto;
        }

        .header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 24px;
          flex-wrap: wrap;
          gap: 16px;
        }

        .back-link {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          text-decoration: none;
          display: inline-block;
          margin-bottom: 8px;
        }

        .back-link:hover {
          color: var(--ppt-color-primary);
        }

        .title {
          font-size: 1.75rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .subtitle {
          font-size: 1rem;
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }

        .invite-button {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 20px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: background 0.2s;
        }

        .invite-button:hover {
          background: var(--ppt-color-primary-hover);
        }

        .tabs {
          display: flex;
          gap: 4px;
          padding: 4px;
          background: var(--ppt-bg-subtle);
          border-radius: 10px;
          margin-bottom: 24px;
          overflow-x: auto;
        }

        .tab {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 10px 16px;
          background: transparent;
          border: none;
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          white-space: nowrap;
          transition: all 0.2s;
        }

        .tab.active {
          background: var(--ppt-bg-surface);
          color: var(--ppt-fg-primary);
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .tab-count {
          padding: 2px 8px;
          background: var(--ppt-border-default);
          border-radius: 10px;
          font-size: 12px;
        }

        .tab.active .tab-count {
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
        }

        .realtor-list {
          display: grid;
          gap: 16px;
        }
      `}</style>
    </div>
  );
}

function RealtorCard({
  realtor,
  agencyId,
  onSelect,
}: {
  realtor: Realtor;
  agencyId: string;
  onSelect: () => void;
}) {
  const t = useTranslations('agency');
  const resendInvitation = useResendInvitation();
  const [resending, setResending] = useState(false);

  const handleResend = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setResending(true);
    try {
      await resendInvitation.mutateAsync({ agencyId, realtorId: realtor.id });
    } finally {
      setResending(false);
    }
  };

  const statusConfig: Record<RealtorStatus, { label: string; color: string; bg: string }> = {
    active: {
      label: t('statusActive'),
      color: 'var(--ppt-color-success)',
      bg: 'var(--ppt-color-success-light)',
    },
    invited: {
      label: t('statusPending'),
      color: 'var(--ppt-color-warning)',
      bg: 'var(--ppt-color-warning-light)',
    },
    inactive: {
      label: t('statusInactive'),
      color: 'var(--ppt-fg-muted)',
      bg: 'var(--ppt-border-default)',
    },
    suspended: {
      label: t('statusSuspended'),
      color: 'var(--ppt-color-danger)',
      bg: 'var(--ppt-color-danger-light)',
    },
  };

  const status = statusConfig[realtor.status];

  return (
    <div
      className="realtor-card"
      onClick={onSelect}
      onKeyDown={(e) => e.key === 'Enter' && onSelect()}
      tabIndex={0}
      role="button"
    >
      <div className="avatar">
        {realtor.photoUrl ? (
          <img src={realtor.photoUrl} alt={realtor.name} />
        ) : (
          <span>{realtor.name.charAt(0)}</span>
        )}
      </div>

      <div className="info">
        <div className="name-row">
          <span className="name">{realtor.name}</span>
          <span className="status" style={{ color: status.color, backgroundColor: status.bg }}>
            {status.label}
          </span>
        </div>
        {realtor.title && <span className="title">{realtor.title}</span>}
        <span className="email">{realtor.email}</span>
      </div>

      <div className="stats">
        <div className="stat">
          <span className="stat-value">{realtor.activeListings}</span>
          <span className="stat-label">{t('cardListings')}</span>
        </div>
        <div className="stat">
          <span className="stat-value">{realtor.totalSales}</span>
          <span className="stat-label">{t('cardSales')}</span>
        </div>
        {realtor.rating && (
          <div className="stat">
            <span className="stat-value rating">
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="var(--ppt-color-warning)"
                aria-hidden="true"
              >
                <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
              </svg>
              {realtor.rating.toFixed(1)}
            </span>
            <span className="stat-label">{t('cardReviews', { count: realtor.reviewCount })}</span>
          </div>
        )}
      </div>

      <div className="actions">
        {realtor.status === 'invited' && (
          <button
            type="button"
            className="action-button"
            onClick={handleResend}
            disabled={resending}
          >
            {resending ? t('buttonSending') : t('buttonResend')}
          </button>
        )}
        <button type="button" className="action-button view" onClick={onSelect}>
          {t('buttonView')}
        </button>
      </div>

      <style jsx>{`
        .realtor-card {
          display: flex;
          align-items: center;
          gap: 16px;
          padding: 20px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          cursor: pointer;
          transition: all 0.2s;
        }

        .realtor-card:hover {
          border-color: var(--ppt-color-primary);
          box-shadow: 0 4px 12px rgba(37, 99, 235, 0.1);
        }

        .avatar {
          width: 56px;
          height: 56px;
          border-radius: 50%;
          background: var(--ppt-color-primary);
          display: flex;
          align-items: center;
          justify-content: center;
          color: var(--ppt-fg-on-accent);
          font-size: 1.25rem;
          font-weight: 600;
          overflow: hidden;
          flex-shrink: 0;
        }

        .avatar img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .info {
          flex: 1;
          min-width: 0;
        }

        .name-row {
          display: flex;
          align-items: center;
          gap: 12px;
          margin-bottom: 4px;
        }

        .name {
          font-size: 1.125rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .status {
          padding: 4px 10px;
          border-radius: 12px;
          font-size: 12px;
          font-weight: 500;
        }

        .title {
          display: block;
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin-bottom: 2px;
        }

        .email {
          font-size: 13px;
          color: var(--ppt-fg-subtle);
        }

        .stats {
          display: flex;
          gap: 24px;
        }

        .stat {
          text-align: center;
        }

        .stat-value {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 4px;
          font-size: 1.25rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .stat-label {
          display: block;
          font-size: 12px;
          color: var(--ppt-fg-muted);
        }

        .actions {
          display: flex;
          gap: 8px;
        }

        .action-button {
          padding: 8px 16px;
          border: 1px solid var(--ppt-border-default);
          background: var(--ppt-bg-surface);
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
          transition: all 0.2s;
        }

        .action-button:hover:not(:disabled) {
          background: var(--ppt-bg-app);
          border-color: var(--ppt-border-strong);
        }

        .action-button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .action-button.view {
          background: var(--ppt-color-primary);
          border-color: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
        }

        .action-button.view:hover {
          background: var(--ppt-color-primary-hover);
        }

        @media (max-width: 768px) {
          .realtor-card {
            flex-wrap: wrap;
          }
          .stats {
            order: 3;
            width: 100%;
            justify-content: flex-start;
            margin-top: 12px;
            padding-top: 12px;
            border-top: 1px solid var(--ppt-bg-subtle);
          }
          .actions {
            order: 2;
          }
        }
      `}</style>
    </div>
  );
}

export function InviteRealtorModal({
  agencyId,
  onClose,
}: {
  agencyId: string;
  onClose: () => void;
}) {
  const [email, setEmail] = useState('');
  const [name, setName] = useState('');
  const [title, setTitle] = useState('');
  const [message, setMessage] = useState('');
  const [inviteError, setInviteError] = useState<string | null>(null);
  const inviteRealtor = useInviteRealtor();
  const t = useTranslations('agency');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setInviteError(null);
    try {
      await inviteRealtor.mutateAsync({
        agencyId,
        data: { email, name, title: title || undefined, message: message || undefined },
      });
      onClose();
    } catch (error) {
      console.error('Failed to invite realtor:', error);
      setInviteError(t('inviteError'));
    }
  };

  return (
    <div
      className="modal-overlay"
      onClick={onClose}
      onKeyDown={(e) => e.key === 'Escape' && onClose()}
      role="presentation"
    >
      <div
        className="modal"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="invite-modal-title"
      >
        <div className="modal-header">
          <h2 id="invite-modal-title">{t('inviteRealtor')}</h2>
          <button
            type="button"
            className="close-button"
            onClick={onClose}
            aria-label={t('buttonClose')}
          >
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          {inviteError && (
            <div className="invite-error" role="alert" aria-live="assertive">
              {inviteError}
            </div>
          )}

          <div className="form-group">
            <label htmlFor="invite-email">{t('formLabelEmail')}</label>
            <input
              id="invite-email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder={t('formPlaceholderEmail')}
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="invite-name">{t('formLabelName')}</label>
            <input
              id="invite-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t('formPlaceholderName')}
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="invite-title">{t('formLabelTitle')}</label>
            <input
              id="invite-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={t('formPlaceholderTitle')}
            />
          </div>

          <div className="form-group">
            <label htmlFor="invite-message">{t('formLabelMessage')}</label>
            <textarea
              id="invite-message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder={t('formPlaceholderMessage')}
              rows={3}
            />
          </div>

          <div className="form-actions">
            <button type="button" className="cancel-button" onClick={onClose}>
              {t('buttonCancel')}
            </button>
            <button type="submit" className="submit-button" disabled={inviteRealtor.isPending}>
              {inviteRealtor.isPending ? t('buttonSending') : t('buttonSendInvitation')}
            </button>
          </div>
        </form>

        <style jsx>{`
          .modal-overlay {
            position: fixed;
            inset: 0;
            background: rgba(0, 0, 0, 0.5);
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 24px;
            z-index: 100;
          }

          .modal {
            background: var(--ppt-bg-surface);
            border-radius: 16px;
            width: 100%;
            max-width: 480px;
            max-height: 90vh;
            overflow-y: auto;
          }

          .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 20px 24px;
            border-bottom: 1px solid var(--ppt-border-default);
          }

          .modal-header h2 {
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--ppt-fg-primary);
            margin: 0;
          }

          .close-button {
            padding: 4px;
            border: none;
            background: transparent;
            color: var(--ppt-fg-muted);
            cursor: pointer;
          }

          form {
            padding: 24px;
          }

          .invite-error {
            margin-bottom: 16px;
            padding: 12px 16px;
            background: var(--ppt-color-danger-light);
            color: var(--ppt-color-danger);
            border: 1px solid var(--ppt-color-danger);
            border-radius: 8px;
            font-size: 14px;
          }

          .form-group {
            margin-bottom: 20px;
          }

          label {
            display: block;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-secondary);
            margin-bottom: 6px;
          }

          input,
          textarea {
            width: 100%;
            padding: 10px 12px;
            border: 1px solid var(--ppt-border-strong);
            border-radius: 8px;
            font-size: 14px;
          }

          input:focus-visible,
          textarea:focus-visible {
            outline: var(--ppt-focus-ring-width) solid var(--ppt-focus-ring-color);
            outline-offset: var(--ppt-focus-ring-offset);
            border-color: var(--ppt-color-primary);
          }

          textarea {
            resize: vertical;
          }

          .form-actions {
            display: flex;
            gap: 12px;
            justify-content: flex-end;
          }

          .cancel-button {
            padding: 10px 20px;
            border: 1px solid var(--ppt-border-strong);
            background: var(--ppt-bg-surface);
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-secondary);
            cursor: pointer;
          }

          .submit-button {
            padding: 10px 20px;
            background: var(--ppt-color-primary);
            border: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-on-accent);
            cursor: pointer;
          }

          .submit-button:disabled {
            opacity: 0.5;
            cursor: not-allowed;
          }
        `}</style>
      </div>
    </div>
  );
}

function RealtorDetailModal({
  realtor,
  agencyId,
  onClose,
}: {
  realtor: Realtor;
  agencyId: string;
  onClose: () => void;
}) {
  const t = useTranslations('agency');
  const updateRealtor = useUpdateRealtor();
  const removeRealtor = useRemoveRealtor();
  const [isEditing, setIsEditing] = useState(false);
  const [title, setTitle] = useState(realtor.title || '');
  const [bio, setBio] = useState(realtor.bio || '');
  const [confirmRemove, setConfirmRemove] = useState(false);

  const handleSave = async () => {
    await updateRealtor.mutateAsync({
      agencyId,
      realtorId: realtor.id,
      data: { title: title || undefined, bio: bio || undefined },
    });
    setIsEditing(false);
  };

  const handleRemove = async () => {
    await removeRealtor.mutateAsync({ agencyId, realtorId: realtor.id });
    onClose();
  };

  const handleStatusChange = async (status: RealtorStatus) => {
    await updateRealtor.mutateAsync({
      agencyId,
      realtorId: realtor.id,
      data: { status },
    });
  };

  return (
    <div
      className="modal-overlay"
      onClick={onClose}
      onKeyDown={(e) => e.key === 'Escape' && onClose()}
      role="presentation"
    >
      <div
        className="modal"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="detail-modal-title"
      >
        <div className="modal-header">
          <h2 id="detail-modal-title">{t('detailTitle')}</h2>
          <button
            type="button"
            className="close-button"
            onClick={onClose}
            aria-label={t('buttonClose')}
          >
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="modal-body">
          {/* Profile Section */}
          <div className="profile-section">
            <div className="avatar">
              {realtor.photoUrl ? (
                <img src={realtor.photoUrl} alt={realtor.name} />
              ) : (
                <span>{realtor.name.charAt(0)}</span>
              )}
            </div>
            <h3>{realtor.name}</h3>
            <p className="email">{realtor.email}</p>
            {realtor.phone && <p className="phone">{realtor.phone}</p>}
          </div>

          {/* Editable Fields */}
          <div className="form-section">
            <div className="form-group">
              <label htmlFor="detail-title">{t('formLabelTitle')}</label>
              <input
                id="detail-title"
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                disabled={!isEditing}
              />
            </div>

            <div className="form-group">
              <label htmlFor="detail-bio">{t('formLabelBio')}</label>
              <textarea
                id="detail-bio"
                value={bio}
                onChange={(e) => setBio(e.target.value)}
                disabled={!isEditing}
                rows={3}
              />
            </div>
          </div>

          {/* Stats */}
          <div className="stats-section">
            <div className="stat">
              <span className="stat-value">{realtor.activeListings}</span>
              <span className="stat-label">{t('statActiveListings')}</span>
            </div>
            <div className="stat">
              <span className="stat-value">{realtor.totalSales}</span>
              <span className="stat-label">{t('statTotalSales')}</span>
            </div>
            <div className="stat">
              <span className="stat-value">{realtor.rating?.toFixed(1) || '-'}</span>
              <span className="stat-label">{t('statRating')}</span>
            </div>
          </div>

          {/* Status Actions */}
          {realtor.status !== 'invited' && (
            <div className="status-section" role="group" aria-labelledby="status-label-text">
              <span id="status-label-text" className="status-label-text">
                {t('statusLabel')}
              </span>
              <div className="status-buttons">
                <button
                  type="button"
                  className={`status-button ${realtor.status === 'active' ? 'active' : ''}`}
                  onClick={() => handleStatusChange('active')}
                >
                  {t('statusActive')}
                </button>
                <button
                  type="button"
                  className={`status-button ${realtor.status === 'inactive' ? 'active' : ''}`}
                  onClick={() => handleStatusChange('inactive')}
                >
                  {t('statusInactive')}
                </button>
                <button
                  type="button"
                  className={`status-button danger ${realtor.status === 'suspended' ? 'active' : ''}`}
                  onClick={() => handleStatusChange('suspended')}
                >
                  {t('statusSuspended')}
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="modal-footer">
          {confirmRemove ? (
            <div className="confirm-remove">
              <span>{t('confirmRemoveQuestion')}</span>
              <button type="button" className="confirm-yes" onClick={handleRemove}>
                {t('confirmRemoveYes')}
              </button>
              <button type="button" className="confirm-no" onClick={() => setConfirmRemove(false)}>
                {t('buttonCancel')}
              </button>
            </div>
          ) : (
            <>
              <button
                type="button"
                className="remove-button"
                onClick={() => setConfirmRemove(true)}
              >
                {t('buttonRemove')}
              </button>
              {isEditing ? (
                <>
                  <button
                    type="button"
                    className="cancel-button"
                    onClick={() => setIsEditing(false)}
                  >
                    {t('buttonCancel')}
                  </button>
                  <button type="button" className="save-button" onClick={handleSave}>
                    {t('buttonSaveChanges')}
                  </button>
                </>
              ) : (
                <button type="button" className="edit-button" onClick={() => setIsEditing(true)}>
                  {t('buttonEdit')}
                </button>
              )}
            </>
          )}
        </div>

        <style jsx>{`
          .modal-overlay {
            position: fixed;
            inset: 0;
            background: rgba(0, 0, 0, 0.5);
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 24px;
            z-index: 100;
          }

          .modal {
            background: var(--ppt-bg-surface);
            border-radius: 16px;
            width: 100%;
            max-width: 560px;
            max-height: 90vh;
            overflow-y: auto;
          }

          .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 20px 24px;
            border-bottom: 1px solid var(--ppt-border-default);
          }

          .modal-header h2 {
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--ppt-fg-primary);
            margin: 0;
          }

          .close-button {
            padding: 4px;
            border: none;
            background: transparent;
            color: var(--ppt-fg-muted);
            cursor: pointer;
          }

          .modal-body {
            padding: 24px;
          }

          .profile-section {
            text-align: center;
            margin-bottom: 24px;
          }

          .avatar {
            width: 80px;
            height: 80px;
            border-radius: 50%;
            background: var(--ppt-color-primary);
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--ppt-fg-on-accent);
            font-size: 2rem;
            font-weight: 600;
            margin: 0 auto 12px;
            overflow: hidden;
          }

          .avatar img {
            width: 100%;
            height: 100%;
            object-fit: cover;
          }

          .profile-section h3 {
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--ppt-fg-primary);
            margin: 0 0 4px;
          }

          .email,
          .phone {
            font-size: 14px;
            color: var(--ppt-fg-muted);
            margin: 0;
          }

          .form-section {
            margin-bottom: 24px;
          }

          .form-group {
            margin-bottom: 16px;
          }

          label {
            display: block;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-secondary);
            margin-bottom: 6px;
          }

          input,
          textarea {
            width: 100%;
            padding: 10px 12px;
            border: 1px solid var(--ppt-border-strong);
            border-radius: 8px;
            font-size: 14px;
          }

          input:disabled,
          textarea:disabled {
            background: var(--ppt-bg-app);
            color: var(--ppt-fg-muted);
          }

          .stats-section {
            display: flex;
            justify-content: center;
            gap: 32px;
            padding: 20px;
            background: var(--ppt-bg-app);
            border-radius: 12px;
            margin-bottom: 24px;
          }

          .stat {
            text-align: center;
          }

          .stat-value {
            display: block;
            font-size: 1.5rem;
            font-weight: 600;
            color: var(--ppt-fg-primary);
          }

          .stat-label {
            font-size: 12px;
            color: var(--ppt-fg-muted);
          }

          .status-section label {
            margin-bottom: 12px;
          }

          .status-buttons {
            display: flex;
            gap: 8px;
          }

          .status-button {
            flex: 1;
            padding: 10px;
            border: 1px solid var(--ppt-border-strong);
            background: var(--ppt-bg-surface);
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-secondary);
            cursor: pointer;
            transition: all 0.2s;
          }

          .status-button.active {
            background: var(--ppt-color-primary);
            border-color: var(--ppt-color-primary);
            color: var(--ppt-fg-on-accent);
          }

          .status-button.danger.active {
            background: var(--ppt-color-danger);
            border-color: var(--ppt-color-danger);
          }

          .modal-footer {
            display: flex;
            gap: 12px;
            padding: 16px 24px;
            border-top: 1px solid var(--ppt-border-default);
            justify-content: flex-end;
          }

          .remove-button {
            margin-right: auto;
            padding: 10px 16px;
            border: 1px solid var(--ppt-color-danger-light);
            background: var(--ppt-color-danger-light);
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-color-danger);
            cursor: pointer;
          }

          .cancel-button,
          .edit-button {
            padding: 10px 20px;
            border: 1px solid var(--ppt-border-strong);
            background: var(--ppt-bg-surface);
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-secondary);
            cursor: pointer;
          }

          .save-button {
            padding: 10px 20px;
            background: var(--ppt-color-primary);
            border: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-on-accent);
            cursor: pointer;
          }

          .confirm-remove {
            display: flex;
            align-items: center;
            gap: 12px;
            width: 100%;
          }

          .confirm-remove span {
            color: var(--ppt-color-danger);
            font-weight: 500;
          }

          .confirm-yes {
            margin-left: auto;
            padding: 10px 16px;
            background: var(--ppt-color-danger);
            border: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-on-accent);
            cursor: pointer;
          }

          .confirm-no {
            padding: 10px 16px;
            border: 1px solid var(--ppt-border-strong);
            background: var(--ppt-bg-surface);
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            color: var(--ppt-fg-secondary);
            cursor: pointer;
          }
        `}</style>
      </div>
    </div>
  );
}

function RealtorListSkeleton() {
  return (
    <div className="skeleton-list">
      {[1, 2, 3].map((i) => (
        <div key={`skel-${i}`} className="skeleton-card" />
      ))}
      <style jsx>{`
        .skeleton-list {
          display: grid;
          gap: 16px;
        }
        .skeleton-card {
          height: 100px;
          background: var(--ppt-border-default);
          border-radius: 12px;
        }
      `}</style>
    </div>
  );
}

function EmptyState({ tab, onInvite }: { tab: TabType; onInvite: () => void }) {
  const t = useTranslations('agency');
  const messages: Record<TabType, { title: string; description: string }> = {
    all: { title: t('emptyAllTitle'), description: t('emptyAllDesc') },
    active: { title: t('emptyActiveTitle'), description: t('emptyActiveDesc') },
    invited: {
      title: t('emptyInvitedTitle'),
      description: t('emptyInvitedDesc'),
    },
    inactive: { title: t('emptyInactiveTitle'), description: t('emptyInactiveDesc') },
  };

  const { title, description } = messages[tab];

  return (
    <div className="empty-state">
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-fg-subtle)"
        strokeWidth="1.5"
        aria-hidden="true"
      >
        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
        <circle cx="9" cy="7" r="4" />
        <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
        <path d="M16 3.13a4 4 0 0 1 0 7.75" />
      </svg>
      <h3>{title}</h3>
      <p>{description}</p>
      {(tab === 'all' || tab === 'invited') && (
        <button type="button" onClick={onInvite}>
          {t('inviteRealtor')}
        </button>
      )}
      <style jsx>{`
        .empty-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 64px 24px;
          text-align: center;
        }
        h3 {
          font-size: 1.25rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }
        p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }
        button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-weight: 500;
          cursor: pointer;
        }
      `}</style>
    </div>
  );
}
