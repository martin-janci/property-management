/**
 * PhotoGallery Component
 *
 * Photo gallery with lightbox for listing detail (Epic 44, Story 44.3).
 */

'use client';

import type { ListingPhoto } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useCallback, useEffect, useState } from 'react';

interface PhotoGalleryProps {
  photos: ListingPhoto[];
  title: string;
}

export function PhotoGallery({ photos, title }: PhotoGalleryProps) {
  const t = useTranslations('gallery');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showLightbox, setShowLightbox] = useState(false);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!showLightbox) return;

      if (e.key === 'Escape') {
        setShowLightbox(false);
      } else if (e.key === 'ArrowLeft') {
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : photos.length - 1));
      } else if (e.key === 'ArrowRight') {
        setSelectedIndex((prev) => (prev < photos.length - 1 ? prev + 1 : 0));
      }
    },
    [showLightbox, photos.length]
  );

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  // Prevent body scroll when lightbox is open
  useEffect(() => {
    if (showLightbox) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [showLightbox]);

  if (photos.length === 0) {
    return (
      <div className="empty-gallery">
        <svg
          width="64"
          height="64"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          aria-hidden="true"
        >
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <circle cx="8.5" cy="8.5" r="1.5" />
          <polyline points="21 15 16 10 5 21" />
        </svg>
        <p>{t('noPhotos')}</p>
        <style jsx>{`
          .empty-gallery {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 400px;
            background: var(--ppt-bg-subtle);
            border-radius: 12px;
            color: var(--ppt-fg-subtle);
          }
          .empty-gallery p {
            margin-top: 16px;
          }
        `}</style>
      </div>
    );
  }

  const mainPhoto = photos[selectedIndex];

  return (
    <>
      <div className="gallery">
        {/* Main Image */}
        <button
          type="button"
          className="main-image-container"
          onClick={() => setShowLightbox(true)}
          aria-label={t('viewFullscreen', { title })}
        >
          <img src={mainPhoto.url} alt={mainPhoto.caption || title} className="main-image" />
          <div className="image-count">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
              <circle cx="8.5" cy="8.5" r="1.5" />
              <polyline points="21 15 16 10 5 21" />
            </svg>
            <span>
              {selectedIndex + 1} / {photos.length}
            </span>
          </div>
        </button>

        {/* Thumbnails */}
        {photos.length > 1 && (
          <div className="thumbnails">
            {photos.slice(0, 5).map((photo, index) => (
              <button
                key={photo.id}
                type="button"
                className={`thumbnail ${index === selectedIndex ? 'active' : ''}`}
                onClick={() => setSelectedIndex(index)}
                aria-label={t('viewPhoto', { number: index + 1 })}
              >
                <img
                  src={photo.thumbnailUrl}
                  alt={photo.caption || t('photoNumber', { number: index + 1 })}
                />
                {index === 4 && photos.length > 5 && (
                  <div className="more-overlay">+{photos.length - 5}</div>
                )}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Lightbox */}
      {showLightbox && (
        <dialog className="lightbox" open aria-label={t('photoGallery')}>
          <button
            type="button"
            className="lightbox-close"
            onClick={() => setShowLightbox(false)}
            aria-label={t('closeGallery')}
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

          <button
            type="button"
            className="lightbox-nav prev"
            onClick={() => setSelectedIndex((prev) => (prev > 0 ? prev - 1 : photos.length - 1))}
            aria-label={t('previousPhoto')}
          >
            <svg
              width="32"
              height="32"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <polyline points="15 18 9 12 15 6" />
            </svg>
          </button>

          <div className="lightbox-content">
            <img
              src={photos[selectedIndex].url}
              alt={
                photos[selectedIndex].caption || t('photoOf', { number: selectedIndex + 1, title })
              }
              className="lightbox-image"
            />
            {photos[selectedIndex].caption && (
              <p className="lightbox-caption">{photos[selectedIndex].caption}</p>
            )}
          </div>

          <button
            type="button"
            className="lightbox-nav next"
            onClick={() => setSelectedIndex((prev) => (prev < photos.length - 1 ? prev + 1 : 0))}
            aria-label={t('nextPhoto')}
          >
            <svg
              width="32"
              height="32"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <polyline points="9 18 15 12 9 6" />
            </svg>
          </button>

          <div className="lightbox-counter">
            {selectedIndex + 1} / {photos.length}
          </div>

          {/* Lightbox Thumbnails */}
          <div className="lightbox-thumbnails">
            {photos.map((photo, index) => (
              <button
                key={photo.id}
                type="button"
                className={`lightbox-thumbnail ${index === selectedIndex ? 'active' : ''}`}
                onClick={() => setSelectedIndex(index)}
                aria-label={t('viewPhoto', { number: index + 1 })}
              >
                <img
                  src={photo.thumbnailUrl}
                  alt={photo.caption || t('photoNumber', { number: index + 1 })}
                />
              </button>
            ))}
          </div>
        </dialog>
      )}

      <style jsx>{`
        .gallery {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .main-image-container {
          position: relative;
          width: 100%;
          aspect-ratio: 16 / 10;
          border-radius: 12px;
          overflow: hidden;
          cursor: pointer;
          border: none;
          padding: 0;
        }

        .main-image {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .image-count {
          position: absolute;
          bottom: 12px;
          right: 12px;
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 8px 12px;
          background: rgba(0, 0, 0, 0.7);
          color: var(--ppt-fg-on-accent);
          border-radius: 20px;
          font-size: 14px;
        }

        .thumbnails {
          display: grid;
          grid-template-columns: repeat(5, 1fr);
          gap: 8px;
        }

        .thumbnail {
          position: relative;
          aspect-ratio: 4 / 3;
          border-radius: 8px;
          overflow: hidden;
          cursor: pointer;
          border: 2px solid transparent;
          padding: 0;
        }

        .thumbnail.active {
          border-color: var(--ppt-color-primary);
        }

        .thumbnail img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .more-overlay {
          position: absolute;
          inset: 0;
          background: rgba(0, 0, 0, 0.6);
          color: var(--ppt-fg-on-accent);
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 18px;
          font-weight: 600;
        }

        /* Lightbox */
        .lightbox {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.95);
          z-index: 100;
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
        }

        .lightbox-close {
          position: absolute;
          top: 16px;
          right: 16px;
          padding: 8px;
          background: transparent;
          border: none;
          color: var(--ppt-fg-on-accent);
          cursor: pointer;
          z-index: 10;
        }

        .lightbox-close:hover {
          color: var(--ppt-border-strong);
        }

        .lightbox-nav {
          position: absolute;
          top: 50%;
          transform: translateY(-50%);
          padding: 16px;
          background: rgba(255, 255, 255, 0.1);
          border: none;
          color: var(--ppt-fg-on-accent);
          cursor: pointer;
          border-radius: 50%;
        }

        .lightbox-nav:hover {
          background: rgba(255, 255, 255, 0.2);
        }

        .lightbox-nav.prev {
          left: 16px;
        }

        .lightbox-nav.next {
          right: 16px;
        }

        .lightbox-content {
          max-width: 90vw;
          max-height: 70vh;
          display: flex;
          flex-direction: column;
          align-items: center;
        }

        .lightbox-image {
          max-width: 100%;
          max-height: 65vh;
          object-fit: contain;
        }

        .lightbox-caption {
          margin-top: 12px;
          color: var(--ppt-border-strong);
          font-size: 14px;
        }

        .lightbox-counter {
          position: absolute;
          top: 16px;
          left: 50%;
          transform: translateX(-50%);
          color: var(--ppt-fg-on-accent);
          font-size: 14px;
        }

        .lightbox-thumbnails {
          position: absolute;
          bottom: 16px;
          left: 50%;
          transform: translateX(-50%);
          display: flex;
          gap: 8px;
          padding: 8px;
          background: rgba(0, 0, 0, 0.5);
          border-radius: 8px;
          max-width: 90vw;
          overflow-x: auto;
        }

        .lightbox-thumbnail {
          width: 60px;
          height: 45px;
          border-radius: 4px;
          overflow: hidden;
          cursor: pointer;
          border: 2px solid transparent;
          padding: 0;
          flex-shrink: 0;
        }

        .lightbox-thumbnail.active {
          border-color: var(--ppt-fg-on-accent);
        }

        .lightbox-thumbnail img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }
      `}</style>
    </>
  );
}
