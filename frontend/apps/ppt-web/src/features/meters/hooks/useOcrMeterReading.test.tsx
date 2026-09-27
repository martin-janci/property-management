/**
 * OCR Meter Reading Hook Tests
 * Epic 128: OCR Meter Preview
 *
 * Regression (#2978): the OCR hooks previously issued a raw `fetch()` with no
 * Authorization header, so every OCR call went out unauthenticated and 401'd
 * in production. They now route through the shared axios client
 * (`getApiClient()`), which stamps `Authorization: Bearer <token>` via its
 * request interceptor. These tests seed a token provider on the configured
 * client and capture the outbound request with a recording adapter to prove the
 * bearer token is attached — they fail on the pre-fix raw-fetch version.
 */
/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosHeaders } from 'axios';
import type { ReactNode } from 'react';
import { configureApiClient, resetApiClient } from '../../../lib/api';
import { useOcrCorrection, useOcrMeterReading, useOcrProcessImage } from './useOcrMeterReading';

const ACCESS_TOKEN = 'ocr-access-token-xyz';

interface RecordedRequest {
  auth: string | undefined;
  url?: string;
  method?: string;
  data: unknown;
}

/** Adapter that records each request and returns a canned response. */
function recordingAdapter(
  responseData: unknown,
  status = 200
): {
  adapter: AxiosAdapter;
  requests: () => RecordedRequest[];
} {
  const seen: RecordedRequest[] = [];
  const adapter: AxiosAdapter = (config) => {
    const headers = AxiosHeaders.from(config.headers);
    seen.push({
      auth: headers.get('Authorization') as string | undefined,
      url: config.url,
      method: config.method,
      data: config.data,
    });
    const response: AxiosResponse = {
      data: responseData,
      status,
      statusText: status >= 400 ? 'Error' : 'OK',
      headers: {},
      config,
    };
    if (status >= 400) {
      return Promise.reject(
        Object.assign(new Error(`Request failed with status code ${status}`), {
          isAxiosError: true,
          response,
          config,
        })
      );
    }
    return Promise.resolve(response);
  };
  return { adapter, requests: () => seen };
}

// Create wrapper with QueryClient
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe('useOcrProcessImage', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('sends the Authorization header and multipart body to /ai/ocr/meter-reading', async () => {
    const mockResult = {
      result: {
        extractedValue: 12345,
        confidence: 0.95,
        boundingBox: { x: 10, y: 20, width: 30, height: 15 },
        rawText: '12345',
        processingTimeMs: 250,
      },
      imageUrl: 'https://example.com/processed.jpg',
    };

    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter(mockResult);
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useOcrProcessImage(), {
      wrapper: createWrapper(),
    });

    const file = new File(['test'], 'meter.jpg', { type: 'image/jpeg' });
    result.current.mutate({ image: file, meterId: 'meter-123' });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockResult);

    const [req] = requests();
    // Core regression assertion: the request carried the bearer token, proving
    // it went through the axios interceptor rather than a raw fetch.
    expect(req.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(req.url).toBe('/ai/ocr/meter-reading');
    expect(req.method).toBe('post');
    expect(req.data).toBeInstanceOf(FormData);
  });

  it('surfaces processing errors', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter } = recordingAdapter({ message: 'Processing failed' }, 500);
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useOcrProcessImage(), {
      wrapper: createWrapper(),
    });

    const file = new File(['test'], 'meter.jpg', { type: 'image/jpeg' });
    result.current.mutate({ image: file, meterId: 'meter-123' });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.error?.message).toBe('Processing failed');
  });
});

describe('useOcrCorrection', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('sends the Authorization header and correction payload to /ai/ocr/correction', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter({});
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useOcrCorrection(), {
      wrapper: createWrapper(),
    });

    result.current.mutate({
      originalValue: 12345,
      correctedValue: 12400,
      imageUrl: 'https://example.com/meter.jpg',
      boundingBox: { x: 10, y: 20, width: 30, height: 15 },
      timestamp: '2024-01-15T10:30:00Z',
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const [req] = requests();
    expect(req.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(req.url).toBe('/ai/ocr/correction');
    expect(req.method).toBe('post');
    expect(JSON.parse(req.data as string)).toEqual({
      original_value: 12345,
      corrected_value: 12400,
      image_url: 'https://example.com/meter.jpg',
      bounding_box: { x: 10, y: 20, width: 30, height: 15 },
      timestamp: '2024-01-15T10:30:00Z',
    });
  });

  it('surfaces correction submission errors', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter } = recordingAdapter({ message: 'Invalid correction data' }, 400);
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useOcrCorrection(), {
      wrapper: createWrapper(),
    });

    result.current.mutate({
      originalValue: 12345,
      correctedValue: 12400,
      imageUrl: 'https://example.com/meter.jpg',
      timestamp: '2024-01-15T10:30:00Z',
    });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.error?.message).toBe('Invalid correction data');
  });
});

describe('useOcrMeterReading', () => {
  beforeEach(() => {
    resetApiClient();
  });

  afterEach(() => {
    resetApiClient();
  });

  it('provides process and correctAndSubmit functions', () => {
    const { result } = renderHook(() => useOcrMeterReading('meter-123'), {
      wrapper: createWrapper(),
    });

    expect(result.current.process).toBeDefined();
    expect(typeof result.current.process).toBe('function');
    expect(result.current.correctAndSubmit).toBeDefined();
    expect(typeof result.current.correctAndSubmit).toBe('function');
  });

  it('provides loading states', () => {
    const { result } = renderHook(() => useOcrMeterReading('meter-123'), {
      wrapper: createWrapper(),
    });

    expect(result.current.isProcessing).toBe(false);
    expect(result.current.isSubmittingCorrection).toBe(false);
  });

  it('provides error states', () => {
    const { result } = renderHook(() => useOcrMeterReading('meter-123'), {
      wrapper: createWrapper(),
    });

    expect(result.current.processingError).toBeNull();
    expect(result.current.correctionError).toBeNull();
  });

  it('provides reset function', () => {
    const { result } = renderHook(() => useOcrMeterReading('meter-123'), {
      wrapper: createWrapper(),
    });

    expect(result.current.reset).toBeDefined();
    expect(typeof result.current.reset).toBe('function');
  });

  it('processes an image through the hook', async () => {
    const mockResult = {
      result: {
        extractedValue: 12345,
        confidence: 0.95,
        boundingBox: { x: 10, y: 20, width: 30, height: 15 },
        rawText: '12345',
        processingTimeMs: 250,
      },
      imageUrl: 'https://example.com/processed.jpg',
    };

    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter } = recordingAdapter(mockResult);
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useOcrMeterReading('meter-123'), {
      wrapper: createWrapper(),
    });

    const file = new File(['test'], 'meter.jpg', { type: 'image/jpeg' });
    const processResult = await result.current.process(file);

    expect(processResult).toEqual(mockResult);
  });

  it('submits a correction through the hook', async () => {
    const instance = configureApiClient({ getToken: () => ACCESS_TOKEN });
    const { adapter, requests } = recordingAdapter({});
    instance.defaults.adapter = adapter;

    const { result } = renderHook(() => useOcrMeterReading('meter-123'), {
      wrapper: createWrapper(),
    });

    await result.current.correctAndSubmit({
      originalValue: 12345,
      correctedValue: 12400,
      imageUrl: 'https://example.com/meter.jpg',
      timestamp: '2024-01-15T10:30:00Z',
    });

    const [req] = requests();
    expect(req.auth).toBe(`Bearer ${ACCESS_TOKEN}`);
    expect(req.url).toBe('/ai/ocr/correction');
  });
});
