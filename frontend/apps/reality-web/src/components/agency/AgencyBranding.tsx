/**
 * AgencyBranding Component
 *
 * Agency branding customization (Epic 45, Story 45.4).
 */

'use client';
import { useAgencyBranding, useMyAgency, useUpdateBranding } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { useCallback, useEffect, useState } from 'react';
import { AgencyLoadError, isNotFoundError, NoAgencyMessage } from './AgencyErrorStates';

export function AgencyBranding() {
  const t = useTranslations('agencyBranding');
  const {
    data: agency,
    isLoading: agencyLoading,
    isError: agencyError,
    error: agencyErrorObj,
    refetch: refetchAgency,
  } = useMyAgency();
  const { data: branding, isLoading } = useAgencyBranding(agency?.id || '');
  const updateBranding = useUpdateBranding();

  const [primaryColor, setPrimaryColor] = useState('#2563eb');
  const [secondaryColor, setSecondaryColor] = useState('#1e40af');
  const [accentColor, setAccentColor] = useState('#10b981');
  const [logoFile, setLogoFile] = useState<File | null>(null);
  const [logoPreview, setLogoPreview] = useState<string | null>(null);
  const [coverFile, setCoverFile] = useState<File | null>(null);
  const [coverPreview, setCoverPreview] = useState<string | null>(null);
  const [hasChanges, setHasChanges] = useState(false);

  // Initialize from branding data
  useEffect(() => {
    if (branding) {
      setPrimaryColor(branding.primaryColor || '#2563eb');
      setSecondaryColor(branding.secondaryColor || '#1e40af');
      setAccentColor(branding.accentColor || '#10b981');
    }
  }, [branding]);

  // Cleanup object URLs to prevent memory leaks
  useEffect(() => {
    return () => {
      if (logoPreview) URL.revokeObjectURL(logoPreview);
      if (coverPreview) URL.revokeObjectURL(coverPreview);
    };
  }, [logoPreview, coverPreview]);

  const handleLogoChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setLogoFile(file);
      setLogoPreview(URL.createObjectURL(file));
      setHasChanges(true);
    }
  }, []);

  const handleCoverChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setCoverFile(file);
      setCoverPreview(URL.createObjectURL(file));
      setHasChanges(true);
    }
  }, []);

  const handleColorChange = useCallback(
    (setter: (c: string) => void) => (e: React.ChangeEvent<HTMLInputElement>) => {
      setter(e.target.value);
      setHasChanges(true);
    },
    []
  );

  const handleSave = async () => {
    if (!agency) return;

    await updateBranding.mutateAsync({
      agencyId: agency.id,
      data: {
        logo: logoFile || undefined,
        coverImage: coverFile || undefined,
        primaryColor,
        secondaryColor,
        accentColor,
      },
    });

    setHasChanges(false);
    setLogoFile(null);
    setCoverFile(null);
  };

  // Distinguish an agency-load failure from a genuine "no agency" state so a
  // transport/server error no longer renders the empty branding form against a
  // missing agency (Issue #2343, sibling of Issue #2277).
  if (agencyError && !isNotFoundError(agencyErrorObj)) {
    return <AgencyLoadError onRetry={() => refetchAgency()} />;
  }
  if (!agencyLoading && !agency) {
    return <NoAgencyMessage />;
  }

  if (agencyLoading || isLoading) {
    return <BrandingSkeleton />;
  }

  return (
    <div className="agency-branding">
      {/* Header */}
      <div className="header">
        <div>
          <Link href="/agency" className="back-link">
            {t('back')}
          </Link>
          <h1 className="title">{t('title')}</h1>
          <p className="subtitle">{t('subtitle')}</p>
        </div>
        <button
          type="button"
          className="save-button"
          onClick={handleSave}
          disabled={!hasChanges || updateBranding.isPending}
        >
          {updateBranding.isPending ? t('saving') : t('saveChanges')}
        </button>
      </div>

      <div className="content-grid">
        {/* Logo Section */}
        <div className="section">
          <h2 className="section-title">{t('logoTitle')}</h2>
          <p className="section-description">{t('logoDesc')}</p>
          <div className="logo-upload">
            <div className="logo-preview">
              {logoPreview || branding?.logoUrl ? (
                <img src={logoPreview || branding?.logoUrl} alt={t('logoAlt')} />
              ) : (
                <div className="logo-placeholder">
                  <svg
                    width="48"
                    height="48"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    aria-hidden="true"
                  >
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                    <circle cx="8.5" cy="8.5" r="1.5" />
                    <polyline points="21 15 16 10 5 21" />
                  </svg>
                </div>
              )}
            </div>
            <div className="upload-actions">
              <label className="upload-button">
                <input type="file" accept="image/*" onChange={handleLogoChange} />
                {t('uploadLogo')}
              </label>
              <span className="upload-hint">{t('logoHint')}</span>
            </div>
          </div>
        </div>

        {/* Cover Image Section */}
        <div className="section">
          <h2 className="section-title">{t('coverTitle')}</h2>
          <p className="section-description">{t('coverDesc')}</p>
          <div className="cover-upload">
            <div className="cover-preview">
              {coverPreview || branding?.coverImageUrl ? (
                <img src={coverPreview || branding?.coverImageUrl} alt={t('coverAlt')} />
              ) : (
                <div className="cover-placeholder">
                  <svg
                    width="48"
                    height="48"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    aria-hidden="true"
                  >
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                    <circle cx="8.5" cy="8.5" r="1.5" />
                    <polyline points="21 15 16 10 5 21" />
                  </svg>
                  <span>{t('noCover')}</span>
                </div>
              )}
            </div>
            <label className="upload-button">
              <input type="file" accept="image/*" onChange={handleCoverChange} />
              {t('uploadCover')}
            </label>
            <span className="upload-hint">{t('coverHint')}</span>
          </div>
        </div>

        {/* Colors Section */}
        <div className="section">
          <h2 className="section-title">{t('colorsTitle')}</h2>
          <p className="section-description">{t('colorsDesc')}</p>

          <div className="colors-grid">
            <div className="color-picker">
              <label htmlFor="primary-color">{t('primaryColor')}</label>
              <div className="color-input-wrapper">
                <input
                  id="primary-color"
                  type="color"
                  value={primaryColor}
                  onChange={handleColorChange(setPrimaryColor)}
                />
                <input
                  type="text"
                  value={primaryColor}
                  onChange={handleColorChange(setPrimaryColor)}
                  pattern="^#[0-9A-Fa-f]{6}$"
                />
              </div>
            </div>

            <div className="color-picker">
              <label htmlFor="secondary-color">{t('secondaryColor')}</label>
              <div className="color-input-wrapper">
                <input
                  id="secondary-color"
                  type="color"
                  value={secondaryColor}
                  onChange={handleColorChange(setSecondaryColor)}
                />
                <input
                  type="text"
                  value={secondaryColor}
                  onChange={handleColorChange(setSecondaryColor)}
                  pattern="^#[0-9A-Fa-f]{6}$"
                />
              </div>
            </div>

            <div className="color-picker">
              <label htmlFor="accent-color">{t('accentColor')}</label>
              <div className="color-input-wrapper">
                <input
                  id="accent-color"
                  type="color"
                  value={accentColor}
                  onChange={handleColorChange(setAccentColor)}
                />
                <input
                  type="text"
                  value={accentColor}
                  onChange={handleColorChange(setAccentColor)}
                  pattern="^#[0-9A-Fa-f]{6}$"
                />
              </div>
            </div>
          </div>
        </div>

        {/* Preview Section */}
        <div className="section preview-section">
          <h2 className="section-title">{t('previewTitle')}</h2>
          <p className="section-description">{t('previewDesc')}</p>

          <div className="preview-card" style={{ borderColor: primaryColor }}>
            <div className="preview-image">
              {branding?.coverImageUrl || coverPreview ? (
                <img src={coverPreview || branding?.coverImageUrl} alt="Preview" />
              ) : (
                <div
                  className="preview-image-placeholder"
                  style={{ backgroundColor: primaryColor }}
                />
              )}
              <span className="preview-badge" style={{ backgroundColor: accentColor }}>
                {t('featured')}
              </span>
            </div>
            <div className="preview-content">
              <div className="preview-header">
                {(logoPreview || branding?.logoUrl) && (
                  <img src={logoPreview || branding?.logoUrl} alt="Logo" className="preview-logo" />
                )}
                <div>
                  <h3 style={{ color: primaryColor }}>{agency?.name || t('yourAgency')}</h3>
                  <p>{t('sampleTitle')}</p>
                </div>
              </div>
              <div className="preview-price" style={{ color: primaryColor }}>
                €450,000
              </div>
              <button
                type="button"
                className="preview-button"
                style={{ backgroundColor: primaryColor }}
              >
                {t('contactAgent')}
              </button>
            </div>
          </div>
        </div>
      </div>

      <style jsx>{`
        .agency-branding {
          padding: 24px;
          max-width: 1200px;
          margin: 0 auto;
        }

        .header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 32px;
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

        .save-button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s;
        }

        .save-button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .save-button:hover:not(:disabled) {
          background: var(--ppt-color-primary-hover);
        }

        .content-grid {
          display: grid;
          gap: 24px;
        }

        @media (min-width: 1024px) {
          .content-grid {
            grid-template-columns: 1fr 1fr;
          }
        }

        .section {
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          padding: 24px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .preview-section {
          grid-column: 1 / -1;
        }

        .section-title {
          font-size: 1.125rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0 0 8px;
        }

        .section-description {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin: 0 0 20px;
        }

        .logo-upload {
          display: flex;
          align-items: center;
          gap: 24px;
        }

        .logo-preview {
          width: 120px;
          height: 60px;
          background: var(--ppt-bg-app);
          border: 2px dashed var(--ppt-border-default);
          border-radius: 8px;
          display: flex;
          align-items: center;
          justify-content: center;
          overflow: hidden;
        }

        .logo-preview img {
          max-width: 100%;
          max-height: 100%;
          object-fit: contain;
        }

        .logo-placeholder {
          color: var(--ppt-fg-subtle);
        }

        .upload-actions {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .upload-button {
          display: inline-block;
          padding: 10px 16px;
          background: var(--ppt-bg-subtle);
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
          transition: background 0.2s;
        }

        .upload-button:hover {
          background: var(--ppt-border-default);
        }

        .upload-button input {
          display: none;
        }

        .upload-hint {
          font-size: 12px;
          color: var(--ppt-fg-subtle);
        }

        .cover-upload {
          display: flex;
          flex-direction: column;
          gap: 16px;
        }

        .cover-preview {
          width: 100%;
          height: 120px;
          background: var(--ppt-bg-app);
          border: 2px dashed var(--ppt-border-default);
          border-radius: 8px;
          display: flex;
          align-items: center;
          justify-content: center;
          overflow: hidden;
        }

        .cover-preview img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .cover-placeholder {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 8px;
          color: var(--ppt-fg-subtle);
          font-size: 14px;
        }

        .colors-grid {
          display: grid;
          gap: 20px;
        }

        @media (min-width: 640px) {
          .colors-grid {
            grid-template-columns: repeat(3, 1fr);
          }
        }

        .color-picker label {
          display: block;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          margin-bottom: 8px;
        }

        .color-input-wrapper {
          display: flex;
          gap: 8px;
          align-items: center;
        }

        .color-input-wrapper input[type='color'] {
          width: 48px;
          height: 40px;
          padding: 2px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 6px;
          cursor: pointer;
        }

        .color-input-wrapper input[type='text'] {
          flex: 1;
          padding: 8px 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 6px;
          font-size: 14px;
          font-family: monospace;
        }

        .preview-card {
          max-width: 360px;
          border: 2px solid;
          border-radius: 12px;
          overflow: hidden;
          background: var(--ppt-bg-surface);
        }

        .preview-image {
          position: relative;
          height: 180px;
          background: var(--ppt-bg-subtle);
        }

        .preview-image img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .preview-image-placeholder {
          width: 100%;
          height: 100%;
          opacity: 0.2;
        }

        .preview-badge {
          position: absolute;
          top: 12px;
          left: 12px;
          padding: 4px 12px;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 600;
          color: var(--ppt-fg-on-accent);
        }

        .preview-content {
          padding: 16px;
        }

        .preview-header {
          display: flex;
          align-items: center;
          gap: 12px;
          margin-bottom: 12px;
        }

        .preview-logo {
          width: 40px;
          height: 20px;
          object-fit: contain;
        }

        .preview-header h3 {
          font-size: 14px;
          font-weight: 600;
          margin: 0;
        }

        .preview-header p {
          font-size: 13px;
          color: var(--ppt-fg-muted);
          margin: 0;
        }

        .preview-price {
          font-size: 1.5rem;
          font-weight: 700;
          margin-bottom: 12px;
        }

        .preview-button {
          width: 100%;
          padding: 10px;
          border: none;
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-on-accent);
          cursor: pointer;
        }
      `}</style>
    </div>
  );
}

function BrandingSkeleton() {
  return (
    <div className="skeleton">
      <div className="skeleton-header" />
      <div className="skeleton-grid">
        <div className="skeleton-card" />
        <div className="skeleton-card" />
        <div className="skeleton-card large" />
      </div>
      <style jsx>{`
        .skeleton {
          padding: 24px;
          max-width: 1200px;
          margin: 0 auto;
        }
        .skeleton-header {
          height: 48px;
          width: 250px;
          background: var(--ppt-border-default);
          border-radius: 8px;
          margin-bottom: 32px;
        }
        .skeleton-grid {
          display: grid;
          gap: 24px;
        }
        @media (min-width: 1024px) {
          .skeleton-grid {
            grid-template-columns: 1fr 1fr;
          }
        }
        .skeleton-card {
          height: 200px;
          background: var(--ppt-border-default);
          border-radius: 12px;
        }
        .skeleton-card.large {
          grid-column: 1 / -1;
        }
      `}</style>
    </div>
  );
}
