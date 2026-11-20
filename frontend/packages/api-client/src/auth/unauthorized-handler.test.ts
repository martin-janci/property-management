/**
 * Unit tests for the unauthorized (401) handler registry.
 *
 * Mirrors token-provider.test.ts — the registry is the generated client's half
 * of the silent refresh-and-replay contract.
 */

import { afterEach, describe, expect, it } from 'vitest';
import {
  clearUnauthorizedHandler,
  getUnauthorizedHandler,
  hasUnauthorizedHandler,
  setUnauthorizedHandler,
} from './unauthorized-handler';

describe('Unauthorized handler registry', () => {
  afterEach(() => {
    clearUnauthorizedHandler();
  });

  it('returns null when no handler is set', () => {
    expect(getUnauthorizedHandler()).toBeNull();
    expect(hasUnauthorizedHandler()).toBe(false);
  });

  it('registers and returns the handler', () => {
    const handler = async () => 'tok';
    setUnauthorizedHandler(handler);
    expect(hasUnauthorizedHandler()).toBe(true);
    expect(getUnauthorizedHandler()).toBe(handler);
  });

  it('allows updating the handler', () => {
    const h1 = async () => 'one';
    const h2 = async () => 'two';
    setUnauthorizedHandler(h1);
    expect(getUnauthorizedHandler()).toBe(h1);
    setUnauthorizedHandler(h2);
    expect(getUnauthorizedHandler()).toBe(h2);
  });

  it('clears the handler', () => {
    setUnauthorizedHandler(async () => 'tok');
    expect(hasUnauthorizedHandler()).toBe(true);
    clearUnauthorizedHandler();
    expect(hasUnauthorizedHandler()).toBe(false);
    expect(getUnauthorizedHandler()).toBeNull();
  });

  it('can clear when nothing is set', () => {
    expect(() => clearUnauthorizedHandler()).not.toThrow();
  });
});
