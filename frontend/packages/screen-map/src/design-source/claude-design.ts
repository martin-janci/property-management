import type { DesignFrame, DesignSource } from './index.js';

class NotImplementedError extends Error {
  constructor(method: string) {
    super(
      `ClaudeDesignAdapter.${method}() is a Phase-2 stub. Implementation deferred until the Claude Design API contract is finalised. See docs/superpowers/specs/2026-05-07-screen-map-system-design.md Section 7.2.`
    );
    this.name = 'NotImplementedError';
  }
}

export class ClaudeDesignAdapter implements DesignSource {
  readonly name = 'claude-design';

  list(): Promise<DesignFrame[]> {
    throw new NotImplementedError('list');
  }

  get(_id: string): Promise<DesignFrame | null> {
    throw new NotImplementedError('get');
  }
}
