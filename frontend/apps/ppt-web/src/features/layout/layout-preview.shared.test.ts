/**
 * Tests for @ppt/shared layout-preview bridge protocol.
 * ADAPT: @ppt/shared has no Vitest setup, so these tests live here in ppt-web
 * and import from @ppt/shared via the workspace alias.
 */

import type { ResolvedScreenLike } from '@ppt/shared';
import {
  connectPreviewChild,
  connectPreviewParent,
  isPreviewChildMessage,
  isPreviewParentMessage,
  LAYOUT_PREVIEW_KIND,
  LAYOUT_PREVIEW_PROTOCOL,
  readPreviewParams,
} from '@ppt/shared';
import { describe, expect, it, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Minimal fake window infrastructure.
//
// makeFakeWindow(origin) — a standalone fake window with its own postMessage
//   spy and listener list.  To wire two windows together for message delivery,
//   call routeMessages(src, srcOrigin, dst) which makes src.postMessage deliver
//   to dst's listeners.
//
// makeFakeWindowPair(originA, originB) — convenience wrapper that returns two
//   pre-wired windows: winA.postMessage delivers to winB (as originA), and
//   winB.postMessage delivers to winA (as originB).
// ---------------------------------------------------------------------------

interface FakeMessageEvent {
  data: unknown;
  origin: string;
}

type MessageListener = (e: FakeMessageEvent) => void;

function makeFakeWindow(origin: string) {
  const listeners: MessageListener[] = [];

  const win = {
    origin,
    listeners, // exposed for direct injection in security-invariant tests
    addEventListener: vi.fn((event: string, cb: MessageListener) => {
      if (event === 'message') listeners.push(cb);
    }),
    removeEventListener: vi.fn((event: string, cb: MessageListener) => {
      if (event === 'message') {
        const idx = listeners.indexOf(cb);
        if (idx !== -1) listeners.splice(idx, 1);
      }
    }),
    postMessage: vi.fn(),
  };

  return win;
}

type FakeWindow = ReturnType<typeof makeFakeWindow>;

/** Make src.postMessage deliver to dst.listeners, stamping srcOrigin on the event. */
function routeMessages(src: FakeWindow, srcOrigin: string, dst: FakeWindow) {
  src.postMessage.mockImplementation((data: unknown, targetOrigin: string) => {
    if (targetOrigin !== '*' && targetOrigin !== dst.origin) return;
    for (const cb of [...dst.listeners]) cb({ data, origin: srcOrigin });
  });
}

function makeFakeWindowPair(originA: string, originB: string) {
  const winA = makeFakeWindow(originA);
  const winB = makeFakeWindow(originB);
  routeMessages(winA, originA, winB);
  routeMessages(winB, originB, winA);
  return { winA, winB, listenersA: winA.listeners, listenersB: winB.listeners };
}

const PARENT_ORIGIN = 'https://parent.example.com';
const CHILD_ORIGIN = 'https://child.example.com';
const FOREIGN_ORIGIN = 'https://evil.example.com';

const SAMPLE_RESOLVED: ResolvedScreenLike = {
  screen: 'dashboard',
  version: 1,
  sections: [{ type: 'header', presentation: 'visible' }],
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
describe('protocol constants', () => {
  it('LAYOUT_PREVIEW_PROTOCOL is 1', () => {
    expect(LAYOUT_PREVIEW_PROTOCOL).toBe(1);
  });
  it('LAYOUT_PREVIEW_KIND is ppt-layout-preview', () => {
    expect(LAYOUT_PREVIEW_KIND).toBe('ppt-layout-preview');
  });
});

// ---------------------------------------------------------------------------
// isPreviewChildMessage
// ---------------------------------------------------------------------------
describe('isPreviewChildMessage', () => {
  it('accepts valid ready message', () => {
    expect(isPreviewChildMessage({ kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'ready' })).toBe(
      true
    );
  });
  it('accepts ready with optional screen', () => {
    expect(
      isPreviewChildMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'ready',
        screen: 'home',
      })
    ).toBe(true);
  });
  it('accepts valid section-click message', () => {
    expect(
      isPreviewChildMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'section-click',
        sectionType: 'header',
      })
    ).toBe(true);
  });
  it('rejects wrong kind', () => {
    expect(isPreviewChildMessage({ kind: 'other', protocol: 1, type: 'ready' })).toBe(false);
  });
  it('rejects wrong protocol', () => {
    expect(isPreviewChildMessage({ kind: LAYOUT_PREVIEW_KIND, protocol: 2, type: 'ready' })).toBe(
      false
    );
  });
  it('rejects unknown type', () => {
    expect(isPreviewChildMessage({ kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'unknown' })).toBe(
      false
    );
  });
  it('rejects null / primitives', () => {
    expect(isPreviewChildMessage(null)).toBe(false);
    expect(isPreviewChildMessage('string')).toBe(false);
    expect(isPreviewChildMessage(42)).toBe(false);
  });
  it('rejects section-click without sectionType', () => {
    expect(
      isPreviewChildMessage({ kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'section-click' })
    ).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isPreviewParentMessage
// ---------------------------------------------------------------------------
describe('isPreviewParentMessage', () => {
  it('accepts valid config message', () => {
    expect(
      isPreviewParentMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'config',
        resolved: SAMPLE_RESOLVED,
      })
    ).toBe(true);
  });
  it('rejects wrong kind', () => {
    expect(
      isPreviewParentMessage({
        kind: 'other',
        protocol: 1,
        type: 'config',
        resolved: SAMPLE_RESOLVED,
      })
    ).toBe(false);
  });
  it('rejects wrong protocol', () => {
    expect(
      isPreviewParentMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 2,
        type: 'config',
        resolved: SAMPLE_RESOLVED,
      })
    ).toBe(false);
  });
  it('rejects missing resolved', () => {
    expect(isPreviewParentMessage({ kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'config' })).toBe(
      false
    );
  });
  it('rejects null / primitives', () => {
    expect(isPreviewParentMessage(null)).toBe(false);
    expect(isPreviewParentMessage(undefined)).toBe(false);
  });
  it('rejects resolved=null', () => {
    expect(
      isPreviewParentMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'config',
        resolved: null,
      })
    ).toBe(false);
  });
  it('rejects resolved=array (bad shape)', () => {
    expect(
      isPreviewParentMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'config',
        resolved: [],
      })
    ).toBe(false);
  });
  it('rejects resolved missing screen string', () => {
    expect(
      isPreviewParentMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'config',
        resolved: { screen: 42, version: 1, sections: [] },
      })
    ).toBe(false);
  });
  it('rejects resolved missing version number', () => {
    expect(
      isPreviewParentMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'config',
        resolved: { screen: 'home', version: '1', sections: [] },
      })
    ).toBe(false);
  });
  it('rejects resolved where sections is not an array', () => {
    expect(
      isPreviewParentMessage({
        kind: LAYOUT_PREVIEW_KIND,
        protocol: 1,
        type: 'config',
        resolved: { screen: 'home', version: 1, sections: null },
      })
    ).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// readPreviewParams
// ---------------------------------------------------------------------------
describe('readPreviewParams', () => {
  it('returns null when layoutPreview flag absent', () => {
    expect(readPreviewParams('?parentOrigin=https://parent.example.com')).toBeNull();
  });
  it('returns null when layoutPreview=0', () => {
    expect(
      readPreviewParams('?layoutPreview=0&parentOrigin=https://parent.example.com')
    ).toBeNull();
  });
  it('returns null when parentOrigin missing', () => {
    expect(readPreviewParams('?layoutPreview=1')).toBeNull();
  });
  it('returns null for garbage parentOrigin', () => {
    expect(readPreviewParams('?layoutPreview=1&parentOrigin=not-a-url')).toBeNull();
  });
  it('returns null for relative URL as parentOrigin', () => {
    expect(readPreviewParams('?layoutPreview=1&parentOrigin=/relative')).toBeNull();
  });
  it('returns { parentOrigin } for valid params', () => {
    const result = readPreviewParams('?layoutPreview=1&parentOrigin=https://parent.example.com');
    expect(result).toEqual({ parentOrigin: 'https://parent.example.com' });
  });
  it('normalizes parentOrigin via URL().origin (strips path/query)', () => {
    const result = readPreviewParams(
      '?layoutPreview=1&parentOrigin=https://parent.example.com/some/path?x=1'
    );
    expect(result).toEqual({ parentOrigin: 'https://parent.example.com' });
  });
  it('returns null for null origin (e.g. file://)', () => {
    // file:// produces origin "null"
    const result = readPreviewParams('?layoutPreview=1&parentOrigin=file:///foo');
    expect(result).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// connectPreviewChild
// ---------------------------------------------------------------------------
describe('connectPreviewChild', () => {
  it('sends ready to parentWindow (not childWin) with correct targetOrigin', () => {
    const childWin = makeFakeWindow(CHILD_ORIGIN);
    const parentWin = makeFakeWindow(PARENT_ORIGIN);
    // Wire parentWin → childWin so parent-sent config reaches childWin listeners
    routeMessages(parentWin, PARENT_ORIGIN, childWin);

    connectPreviewChild({
      parentOrigin: PARENT_ORIGIN,
      win: childWin as unknown as Window,
      parentWindow: parentWin as unknown as Pick<Window, 'postMessage'>,
      onConfig: vi.fn(),
    });

    // parentWin.postMessage must have been called (not childWin)
    expect(parentWin.postMessage).toHaveBeenCalledOnce();
    expect(childWin.postMessage).not.toHaveBeenCalled();

    const [msg, targetOrigin] = parentWin.postMessage.mock.calls[0];
    expect(targetOrigin).toBe(PARENT_ORIGIN); // never '*'
    expect(msg).toMatchObject({
      kind: LAYOUT_PREVIEW_KIND,
      protocol: 1,
      type: 'ready',
    });
  });

  it('sends ready with screen when provided', () => {
    const childWin = makeFakeWindow(CHILD_ORIGIN);
    const parentWin = makeFakeWindow(PARENT_ORIGIN);
    connectPreviewChild({
      parentOrigin: PARENT_ORIGIN,
      screen: 'home',
      win: childWin as unknown as Window,
      parentWindow: parentWin as unknown as Pick<Window, 'postMessage'>,
      onConfig: vi.fn(),
    });
    const [msg] = parentWin.postMessage.mock.calls[0];
    expect(msg).toMatchObject({ type: 'ready', screen: 'home' });
    expect(childWin.postMessage).not.toHaveBeenCalled();
  });

  it('delivers config callback for messages from parentOrigin', () => {
    const childWin = makeFakeWindow(CHILD_ORIGIN);
    const parentWin = makeFakeWindow(PARENT_ORIGIN);
    // parentWin.postMessage → childWin listeners (for incoming config)
    routeMessages(parentWin, PARENT_ORIGIN, childWin);

    const onConfig = vi.fn();
    connectPreviewChild({
      parentOrigin: PARENT_ORIGIN,
      win: childWin as unknown as Window,
      parentWindow: parentWin as unknown as Pick<Window, 'postMessage'>,
      onConfig,
    });
    // Parent sends config to childWin
    parentWin.postMessage(
      { kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'config', resolved: SAMPLE_RESOLVED },
      CHILD_ORIGIN
    );
    expect(onConfig).toHaveBeenCalledOnce();
    expect(onConfig).toHaveBeenCalledWith(SAMPLE_RESOLVED);
  });

  it('ignores config from foreign origin (security invariant)', () => {
    const childWin = makeFakeWindow(CHILD_ORIGIN);
    const parentWin = makeFakeWindow(PARENT_ORIGIN);
    const onConfig = vi.fn();
    connectPreviewChild({
      parentOrigin: PARENT_ORIGIN,
      win: childWin as unknown as Window,
      parentWindow: parentWin as unknown as Pick<Window, 'postMessage'>,
      onConfig,
    });
    // Inject message directly with a foreign origin
    for (const cb of childWin.listeners) {
      cb({
        data: { kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'config', resolved: SAMPLE_RESOLVED },
        origin: FOREIGN_ORIGIN,
      });
    }
    expect(onConfig).not.toHaveBeenCalled();
  });

  it('ignores malformed messages (silently)', () => {
    const childWin = makeFakeWindow(CHILD_ORIGIN);
    const parentWin = makeFakeWindow(PARENT_ORIGIN);
    const onConfig = vi.fn();
    connectPreviewChild({
      parentOrigin: PARENT_ORIGIN,
      win: childWin as unknown as Window,
      parentWindow: parentWin as unknown as Pick<Window, 'postMessage'>,
      onConfig,
    });
    for (const cb of childWin.listeners) {
      cb({ data: { kind: 'other', type: 'config' }, origin: PARENT_ORIGIN });
      cb({ data: null, origin: PARENT_ORIGIN });
      cb({ data: 42, origin: PARENT_ORIGIN });
    }
    expect(onConfig).not.toHaveBeenCalled();
  });

  it('sendSectionClick posts to parentWindow (not childWin), targetOrigin=parentOrigin', () => {
    const childWin = makeFakeWindow(CHILD_ORIGIN);
    const parentWin = makeFakeWindow(PARENT_ORIGIN);
    const { sendSectionClick } = connectPreviewChild({
      parentOrigin: PARENT_ORIGIN,
      win: childWin as unknown as Window,
      parentWindow: parentWin as unknown as Pick<Window, 'postMessage'>,
      onConfig: vi.fn(),
    });
    sendSectionClick('header');
    // parentWin.postMessage called twice: once for ready, once for section-click
    const calls = parentWin.postMessage.mock.calls;
    expect(calls).toHaveLength(2);
    const [msg, targetOrigin] = calls[1];
    expect(targetOrigin).toBe(PARENT_ORIGIN);
    expect(msg).toMatchObject({
      kind: LAYOUT_PREVIEW_KIND,
      protocol: 1,
      type: 'section-click',
      sectionType: 'header',
    });
    // childWin must never have been used for outbound messages
    expect(childWin.postMessage).not.toHaveBeenCalled();
  });

  it('dispose removes the message listener from win (not parentWindow)', () => {
    const childWin = makeFakeWindow(CHILD_ORIGIN);
    const parentWin = makeFakeWindow(PARENT_ORIGIN);
    const { dispose } = connectPreviewChild({
      parentOrigin: PARENT_ORIGIN,
      win: childWin as unknown as Window,
      parentWindow: parentWin as unknown as Pick<Window, 'postMessage'>,
      onConfig: vi.fn(),
    });
    expect(childWin.addEventListener).toHaveBeenCalledOnce();
    dispose();
    expect(childWin.removeEventListener).toHaveBeenCalledOnce();
    // same callback reference used for add and remove
    const addCb = childWin.addEventListener.mock.calls[0][1];
    const removeCb = childWin.removeEventListener.mock.calls[0][1];
    expect(addCb).toBe(removeCb);
  });
});

// ---------------------------------------------------------------------------
// connectPreviewParent
// ---------------------------------------------------------------------------
describe('connectPreviewParent', () => {
  it('calls onReady when child sends ready from childOrigin', () => {
    const { winA: parentWin, winB: childWin } = makeFakeWindowPair(PARENT_ORIGIN, CHILD_ORIGIN);
    const onReady = vi.fn();
    connectPreviewParent({
      childWindow: () => childWin as unknown as Window,
      childOrigin: CHILD_ORIGIN,
      onReady,
      onSectionClick: vi.fn(),
      win: parentWin as unknown as Window,
    });
    // child sends ready
    childWin.postMessage({ kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'ready' }, PARENT_ORIGIN);
    expect(onReady).toHaveBeenCalledOnce();
  });

  it('calls onSectionClick when child sends section-click from childOrigin', () => {
    const { winA: parentWin, winB: childWin } = makeFakeWindowPair(PARENT_ORIGIN, CHILD_ORIGIN);
    const onSectionClick = vi.fn();
    connectPreviewParent({
      childWindow: () => childWin as unknown as Window,
      childOrigin: CHILD_ORIGIN,
      onReady: vi.fn(),
      onSectionClick,
      win: parentWin as unknown as Window,
    });
    childWin.postMessage(
      { kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'section-click', sectionType: 'gallery' },
      PARENT_ORIGIN
    );
    expect(onSectionClick).toHaveBeenCalledWith('gallery');
  });

  it('ignores ready/section-click from foreign origin (security invariant)', () => {
    const { winA: parentWin, listenersA } = makeFakeWindowPair(PARENT_ORIGIN, CHILD_ORIGIN);
    const onReady = vi.fn();
    const onSectionClick = vi.fn();
    connectPreviewParent({
      childWindow: () => null,
      childOrigin: CHILD_ORIGIN,
      onReady,
      onSectionClick,
      win: parentWin as unknown as Window,
    });
    for (const cb of listenersA) {
      cb({
        data: { kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'ready' },
        origin: FOREIGN_ORIGIN,
      });
      cb({
        data: { kind: LAYOUT_PREVIEW_KIND, protocol: 1, type: 'section-click', sectionType: 'x' },
        origin: FOREIGN_ORIGIN,
      });
    }
    expect(onReady).not.toHaveBeenCalled();
    expect(onSectionClick).not.toHaveBeenCalled();
  });

  it('ignores malformed messages from childOrigin', () => {
    const { winA: parentWin, listenersA } = makeFakeWindowPair(PARENT_ORIGIN, CHILD_ORIGIN);
    const onReady = vi.fn();
    connectPreviewParent({
      childWindow: () => null,
      childOrigin: CHILD_ORIGIN,
      onReady,
      onSectionClick: vi.fn(),
      win: parentWin as unknown as Window,
    });
    for (const cb of listenersA) {
      cb({ data: { kind: 'other', type: 'ready' }, origin: CHILD_ORIGIN });
      cb({ data: null, origin: CHILD_ORIGIN });
    }
    expect(onReady).not.toHaveBeenCalled();
  });

  it('sendConfig posts to child window with childOrigin as targetOrigin (never *)', () => {
    const { winA: parentWin, winB: childWin } = makeFakeWindowPair(PARENT_ORIGIN, CHILD_ORIGIN);
    const { sendConfig } = connectPreviewParent({
      childWindow: () => childWin as unknown as Window,
      childOrigin: CHILD_ORIGIN,
      onReady: vi.fn(),
      onSectionClick: vi.fn(),
      win: parentWin as unknown as Window,
    });
    sendConfig(SAMPLE_RESOLVED);
    expect(childWin.postMessage).toHaveBeenCalledOnce();
    const [msg, targetOrigin] = childWin.postMessage.mock.calls[0];
    expect(targetOrigin).toBe(CHILD_ORIGIN); // never '*'
    expect(msg).toMatchObject({
      kind: LAYOUT_PREVIEW_KIND,
      protocol: 1,
      type: 'config',
      resolved: SAMPLE_RESOLVED,
    });
  });

  it('sendConfig is a no-op when childWindow returns null', () => {
    const { winA: parentWin } = makeFakeWindowPair(PARENT_ORIGIN, CHILD_ORIGIN);
    const { sendConfig } = connectPreviewParent({
      childWindow: () => null,
      childOrigin: CHILD_ORIGIN,
      onReady: vi.fn(),
      onSectionClick: vi.fn(),
      win: parentWin as unknown as Window,
    });
    // Must not throw
    expect(() => sendConfig(SAMPLE_RESOLVED)).not.toThrow();
  });

  it('dispose removes the message listener', () => {
    const { winA: parentWin } = makeFakeWindowPair(PARENT_ORIGIN, CHILD_ORIGIN);
    const { dispose } = connectPreviewParent({
      childWindow: () => null,
      childOrigin: CHILD_ORIGIN,
      onReady: vi.fn(),
      onSectionClick: vi.fn(),
      win: parentWin as unknown as Window,
    });
    expect(parentWin.addEventListener).toHaveBeenCalledOnce();
    dispose();
    expect(parentWin.removeEventListener).toHaveBeenCalledOnce();
    const addCb = parentWin.addEventListener.mock.calls[0][1];
    const removeCb = parentWin.removeEventListener.mock.calls[0][1];
    expect(addCb).toBe(removeCb);
  });
});
