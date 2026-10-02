/**
 * OCR Meter Reading Hook
 * Epic 128: OCR Meter Preview
 *
 * Handles OCR processing and correction feedback.
 */

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { getApiClient } from '../../../lib/api';
import type { OcrCorrection, OcrResult } from '../components/OcrPreviewCard';

// Path is relative to the api-client baseURL (`/api/v1`). Routing through
// `getApiClient()` ensures the shared axios interceptors apply — most
// importantly Bearer-token injection, so these calls are authenticated. A raw
// `fetch()` here would omit the Authorization header and 401 in prod. For the
// multipart upload, axios detects the `FormData` body and sets the correct
// `multipart/form-data` Content-Type (with boundary) itself.
const API_BASE = '/ai';

/** Request to process meter image with OCR */
interface OcrProcessRequest {
  image: File;
  meterId: string;
}

/** Response from OCR processing */
interface OcrProcessResponse {
  result: OcrResult;
  imageUrl: string;
}

/** Process meter image with OCR */
export function useOcrProcessImage() {
  return useMutation({
    mutationFn: async ({ image, meterId }: OcrProcessRequest): Promise<OcrProcessResponse> => {
      const formData = new FormData();
      formData.append('image', image);
      formData.append('meter_id', meterId);

      // Set a multipart Content-Type so axios keeps the FormData body as-is:
      // the client's default `application/json` would otherwise make axios
      // serialize the FormData to JSON (via formDataToJSON) instead of sending
      // a multipart upload. The browser/adapter replaces this with the real
      // boundary at send time.
      const res = await getApiClient().post<OcrProcessResponse>(
        `${API_BASE}/ocr/meter-reading`,
        formData,
        { headers: { 'Content-Type': 'multipart/form-data' } }
      );
      return res.data;
    },
  });
}

/** Submit OCR correction for model training */
export function useOcrCorrection() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (correction: OcrCorrection): Promise<void> => {
      await getApiClient().post<void>(`${API_BASE}/ocr/correction`, {
        original_value: correction.originalValue,
        corrected_value: correction.correctedValue,
        image_url: correction.imageUrl,
        bounding_box: correction.boundingBox,
        timestamp: correction.timestamp,
      });
    },
    onSuccess: () => {
      // Optionally invalidate any OCR-related caches
      queryClient.invalidateQueries({ queryKey: ['ocr'] });
    },
  });
}

/** Hook to handle the complete OCR flow */
export function useOcrMeterReading(meterId: string) {
  const processImage = useOcrProcessImage();
  const submitCorrection = useOcrCorrection();

  const process = async (image: File) => {
    return processImage.mutateAsync({ image, meterId });
  };

  const correctAndSubmit = async (correction: OcrCorrection) => {
    await submitCorrection.mutateAsync(correction);
  };

  return {
    process,
    correctAndSubmit,
    isProcessing: processImage.isPending,
    isSubmittingCorrection: submitCorrection.isPending,
    processingError: processImage.error,
    correctionError: submitCorrection.error,
    reset: () => {
      processImage.reset();
      submitCorrection.reset();
    },
  };
}
