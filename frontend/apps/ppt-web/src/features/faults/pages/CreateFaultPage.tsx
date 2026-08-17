/**
 * CreateFaultPage - page for creating a new fault report.
 * Epic 4: Fault Reporting & Resolution (UC-03.1)
 * Epic 126: AI-Enhanced Fault Reporting (Story 126.1 - Photo-First)
 */

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type AiSuggestion, FaultForm, type FaultFormData } from '../components/FaultForm';
import type { UploadedPhoto } from '../components/PhotoUploader';

interface CreateFaultPageProps {
  buildings: Array<{ id: string; name: string }>;
  units: Array<{ id: string; designation: string }>;
  isSubmitting?: boolean;
  onSubmit: (data: FaultFormData) => void;
  onCancel: () => void;
  /** Enable photo-first mode with AI suggestions (Epic 126) */
  enablePhotoFirst?: boolean;
}

/**
 * Photo-first AI triage has no production endpoint yet: the only backend
 * suggestion route (`POST /api/v1/faults/{id}/suggest`) analyses an
 * already-created fault's title/description text, not the pre-creation photos
 * this page uploads. Rather than fabricate a hardcoded suggestion for real
 * users, the mock is gated behind an explicit dev-only opt-in flag and the UI
 * shows a clear "unavailable" state otherwise. Never ship fake AI output.
 */
const MOCK_AI_TRIAGE_ENABLED =
  import.meta.env.DEV && import.meta.env.VITE_ENABLE_MOCK_AI_TRIAGE === 'true';

export function CreateFaultPage({
  buildings,
  units,
  isSubmitting,
  onSubmit,
  onCancel,
  enablePhotoFirst = true,
}: CreateFaultPageProps) {
  const { t } = useTranslation();
  const [aiSuggestion, setAiSuggestion] = useState<AiSuggestion | null>(null);
  const [aiSuggestionLoading, setAiSuggestionLoading] = useState(false);
  const [aiSuggestionUnavailable, setAiSuggestionUnavailable] = useState(false);

  const handlePhotosChange = useCallback((photos: UploadedPhoto[]) => {
    const hasPendingPhotos =
      photos.length > 0 && photos.some((p) => p.status === 'pending' || p.status === 'uploaded');

    if (!hasPendingPhotos) {
      setAiSuggestion(null);
      setAiSuggestionLoading(false);
      setAiSuggestionUnavailable(false);
      return;
    }

    // No production endpoint exists for pre-creation photo-based AI triage
    // (see MOCK_AI_TRIAGE_ENABLED above). Surface an honest "unavailable"
    // state instead of fabricating a suggestion.
    if (!MOCK_AI_TRIAGE_ENABLED) {
      setAiSuggestion(null);
      setAiSuggestionLoading(false);
      setAiSuggestionUnavailable(true);
      return;
    }

    // Dev-only mock (explicitly opted in via VITE_ENABLE_MOCK_AI_TRIAGE).
    setAiSuggestionUnavailable(false);
    setAiSuggestionLoading(true);
    setTimeout(() => {
      setAiSuggestion({
        category: 'plumbing',
        confidence: 0.85,
        priority: 'medium',
      });
      setAiSuggestionLoading(false);
    }, 1500);
  }, []);

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <div className="mb-6">
        <button
          type="button"
          onClick={onCancel}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          ← {t('common.backToFaults')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('faults.reportFault')}</h1>
        <p className="text-gray-600 mt-2">
          {enablePhotoFirst
            ? t('faults.photo.sectionDescription')
            : "Describe the issue you're experiencing. Include as much detail as possible to help us resolve it quickly."}
        </p>
      </div>

      <div className="bg-white rounded-lg shadow p-6">
        <FaultForm
          buildings={buildings}
          units={units}
          isSubmitting={isSubmitting}
          onSubmit={onSubmit}
          onCancel={onCancel}
          enablePhotoFirst={enablePhotoFirst}
          aiSuggestion={aiSuggestion}
          aiSuggestionLoading={aiSuggestionLoading}
          aiSuggestionUnavailable={aiSuggestionUnavailable}
          onPhotosChange={handlePhotosChange}
        />
      </div>
    </div>
  );
}
