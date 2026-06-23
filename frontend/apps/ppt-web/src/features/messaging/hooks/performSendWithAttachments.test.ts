/// <reference types="vitest/globals" />
/**
 * Unit tests for the message-attachment send orchestration (UC-05.9, BIT-208).
 *
 * Verifies the multi-step flow ordering — send the message first, then for each
 * attachment request a presigned URL, PUT the bytes, and link the object — and
 * that an upload failure aborts before linking.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { performSendWithAttachments } from './useMessaging';

type Api = Parameters<typeof performSendWithAttachments>[0];

function makeFile(name: string, type: string, size: number): File {
  const file = new File(['x'], name, { type });
  Object.defineProperty(file, 'size', { value: size });
  return file;
}

function makeApi(overrides: Partial<Api> = {}) {
  const calls: string[] = [];
  const api = {
    sendMessage: vi.fn(async () => {
      calls.push('send');
      return { message: 'ok', sentMessage: { id: 'msg-1' } };
    }),
    requestAttachmentUploadUrl: vi.fn(async () => {
      calls.push('upload-url');
      return { url: 'https://s3.example/put', expiresAt: 'soon', fileKey: 'key-1' };
    }),
    linkMessageAttachment: vi.fn(async () => {
      calls.push('link');
      return { id: 'att-1' };
    }),
    ...overrides,
  } as unknown as Api;
  return { api, calls };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('performSendWithAttachments (UC-05.9)', () => {
  it('sends a plain message with no attachments and never touches the attachment API', async () => {
    const { api, calls } = makeApi();
    const fetchSpy = vi.spyOn(globalThis, 'fetch');

    const res = await performSendWithAttachments(api, 'thread-1', { content: 'hi' });

    expect(res.sentMessage.id).toBe('msg-1');
    expect(calls).toEqual(['send']);
    expect(api.requestAttachmentUploadUrl).not.toHaveBeenCalled();
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('sends, then per attachment requests a URL, PUTs the bytes, and links — in order', async () => {
    const { api, calls } = makeApi();
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue({ ok: true, status: 200 } as Response);
    const file = makeFile('photo.png', 'image/png', 1234);

    await performSendWithAttachments(api, 'thread-1', {
      content: 'see attached',
      attachments: [{ id: 'p1', file, previewUrl: 'blob:x' }],
    });

    expect(calls).toEqual(['send', 'upload-url', 'link']);
    expect(api.requestAttachmentUploadUrl).toHaveBeenCalledWith('thread-1', {
      fileName: 'photo.png',
      fileType: 'image/png',
      fileSize: 1234,
    });
    expect(fetchSpy).toHaveBeenCalledWith(
      'https://s3.example/put',
      expect.objectContaining({ method: 'PUT', body: file })
    );
    expect(api.linkMessageAttachment).toHaveBeenCalledWith('thread-1', 'msg-1', {
      fileKey: 'key-1',
      fileName: 'photo.png',
      fileType: 'image/png',
      fileSize: 1234,
    });
  });

  it('throws and does not link when the byte upload fails', async () => {
    const { api } = makeApi();
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: false, status: 403 } as Response);
    const file = makeFile('doc.pdf', 'application/pdf', 10);

    await expect(
      performSendWithAttachments(api, 'thread-1', {
        content: '',
        attachments: [{ id: 'p1', file, previewUrl: '' }],
      })
    ).rejects.toThrow(/Failed to upload attachment/);

    expect(api.linkMessageAttachment).not.toHaveBeenCalled();
  });
});
