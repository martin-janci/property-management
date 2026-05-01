/**
 * ContactForm Component
 *
 * Contact form for listing inquiries (Epic 44, Story 44.3 & 44.6).
 */

'use client';

import type { CreateInquiryRequest, InquiryType, ListingAgent } from '@ppt/reality-api-client';
import { useCreateInquiry } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useState } from 'react';

interface ContactFormProps {
  listingId: string;
  agent: ListingAgent;
}

export function ContactForm({ listingId, agent }: ContactFormProps) {
  const t = useTranslations('contact');
  const [formData, setFormData] = useState({
    name: '',
    email: '',
    phone: '',
    message: '',
    inquiryType: 'general' as InquiryType,
  });
  const [showSuccess, setShowSuccess] = useState(false);

  const createInquiry = useCreateInquiry();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const request: CreateInquiryRequest = {
      listingId,
      type: formData.inquiryType,
      message: formData.message,
      name: formData.name,
      email: formData.email,
      phone: formData.phone || undefined,
    };

    try {
      await createInquiry.mutateAsync(request);
      setShowSuccess(true);
      setFormData({
        name: '',
        email: '',
        phone: '',
        message: '',
        inquiryType: 'general',
      });
    } catch {
      // Error handled by mutation state
    }
  };

  if (showSuccess) {
    return (
      <div className="contact-form success">
        <div className="success-icon">
          <svg
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="10" />
            <path d="M9 12l2 2 4-4" />
          </svg>
        </div>
        <h3 className="success-title">{t('messageSent')}</h3>
        <p className="success-text">{t('inquirySent')}</p>
        <button type="button" className="new-message-button" onClick={() => setShowSuccess(false)}>
          {t('sendAnother')}
        </button>
        <style jsx>{`
          .contact-form.success {
            background: var(--ppt-bg-surface);
            border-radius: 12px;
            padding: 32px;
            text-align: center;
          }
          .success-icon {
            color: var(--ppt-color-success);
            margin-bottom: 16px;
          }
          .success-title {
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--ppt-fg-primary);
            margin: 0 0 8px;
          }
          .success-text {
            color: var(--ppt-fg-muted);
            margin: 0 0 24px;
          }
          .new-message-button {
            padding: 8px 16px;
            background: transparent;
            border: 1px solid var(--ppt-border-default);
            border-radius: 8px;
            font-size: 14px;
            color: var(--ppt-fg-secondary);
            cursor: pointer;
          }
          .new-message-button:hover {
            background: var(--ppt-bg-app);
          }
        `}</style>
      </div>
    );
  }

  return (
    <div className="contact-form">
      {/* Agent Info */}
      <div className="agent-info">
        <div className="agent-avatar">
          {agent.avatarUrl ? (
            <img src={agent.avatarUrl} alt={agent.name} />
          ) : (
            <span>{agent.name.charAt(0).toUpperCase()}</span>
          )}
        </div>
        <div className="agent-details">
          <p className="agent-name">{agent.name}</p>
          {agent.agencyName && <p className="agent-agency">{agent.agencyName}</p>}
        </div>
      </div>

      {/* Form */}
      <form onSubmit={handleSubmit}>
        <div className="form-group">
          <label htmlFor="inquiryType" className="form-label">
            {t('iWantTo')}
          </label>
          <select
            id="inquiryType"
            className="form-select"
            value={formData.inquiryType}
            onChange={(e) =>
              setFormData({ ...formData, inquiryType: e.target.value as InquiryType })
            }
          >
            <option value="general">{t('askQuestion')}</option>
            <option value="viewing_request">{t('scheduleViewing')}</option>
            <option value="price_negotiation">{t('discussPrice')}</option>
            <option value="availability">{t('checkAvailability')}</option>
          </select>
        </div>

        <div className="form-group">
          <label htmlFor="name" className="form-label">
            {t('name')}
          </label>
          <input
            id="name"
            type="text"
            className="form-input"
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            required
          />
        </div>

        <div className="form-group">
          <label htmlFor="email" className="form-label">
            {t('email')}
          </label>
          <input
            id="email"
            type="email"
            className="form-input"
            value={formData.email}
            onChange={(e) => setFormData({ ...formData, email: e.target.value })}
            required
          />
        </div>

        <div className="form-group">
          <label htmlFor="phone" className="form-label">
            {t('phone')} <span className="optional">{t('optional')}</span>
          </label>
          <input
            id="phone"
            type="tel"
            className="form-input"
            value={formData.phone}
            onChange={(e) => setFormData({ ...formData, phone: e.target.value })}
          />
        </div>

        <div className="form-group">
          <label htmlFor="message" className="form-label">
            {t('message')}
          </label>
          <textarea
            id="message"
            className="form-textarea"
            rows={4}
            value={formData.message}
            onChange={(e) => setFormData({ ...formData, message: e.target.value })}
            placeholder={t('placeholder')}
            required
          />
        </div>

        {createInquiry.error && <div className="error-message">{t('failedToSend')}</div>}

        <button type="submit" className="submit-button" disabled={createInquiry.isPending}>
          {createInquiry.isPending ? t('sending') : t('sendMessage')}
        </button>

        <p className="privacy-notice">{t('privacyNotice')}</p>
      </form>

      <style jsx>{`
        .contact-form {
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          padding: 24px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .agent-info {
          display: flex;
          align-items: center;
          gap: 12px;
          padding-bottom: 16px;
          margin-bottom: 16px;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .agent-avatar {
          width: 48px;
          height: 48px;
          border-radius: 50%;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 18px;
          font-weight: 600;
          overflow: hidden;
        }

        .agent-avatar img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .agent-name {
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .agent-agency {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin: 2px 0 0;
        }

        .form-group {
          margin-bottom: 16px;
        }

        .form-label {
          display: block;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          margin-bottom: 6px;
        }

        .optional {
          font-weight: 400;
          color: var(--ppt-fg-subtle);
        }

        .form-input,
        .form-select,
        .form-textarea {
          width: 100%;
          padding: 10px 12px;
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          font-size: 14px;
          color: var(--ppt-fg-primary);
        }

        .form-input:focus-visible,
        .form-select:focus-visible,
        .form-textarea:focus-visible {
          outline: var(--ppt-focus-ring-width) solid var(--ppt-focus-ring-color);
          outline-offset: var(--ppt-focus-ring-offset);
          border-color: var(--ppt-color-primary);
        }

        .form-textarea {
          resize: vertical;
          min-height: 100px;
        }

        .error-message {
          padding: 12px;
          background: var(--ppt-color-danger-light);
          border: 1px solid #fecaca;
          border-radius: 8px;
          color: var(--ppt-color-danger-hover);
          font-size: 14px;
          margin-bottom: 16px;
        }

        .submit-button {
          width: 100%;
          padding: 12px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 16px;
          font-weight: 600;
          cursor: pointer;
          transition: background 0.2s;
        }

        .submit-button:hover:not(:disabled) {
          background: var(--ppt-color-primary-hover);
        }

        .submit-button:disabled {
          opacity: 0.7;
          cursor: not-allowed;
        }

        .privacy-notice {
          font-size: 12px;
          color: var(--ppt-fg-subtle);
          text-align: center;
          margin: 12px 0 0;
        }
      `}</style>
    </div>
  );
}
