import { describe, expect, it } from 'vitest';
import { sanitize } from './index';

describe('sanitize', () => {
  it('lowercases and replaces non-alphanum', () => {
    expect(sanitize('feature/UC-14')).toBe('feature-uc-14');
    expect(sanitize('hotfix/Critical Fix')).toBe('hotfix-critical-fix');
  });
  it('strips leading/trailing dashes', () => {
    expect(sanitize('///---bad---///')).toBe('bad');
  });
});
