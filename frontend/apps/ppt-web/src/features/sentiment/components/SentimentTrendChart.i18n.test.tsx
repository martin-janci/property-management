/// <reference types="vitest/globals" />
/**
 * Regression test for the Sentiment trend spike marker's screen-reader label.
 *
 * Before the fix the spike `<circle>` rendered a hardcoded English
 * `aria-label={`Spike on ${date}`}` while every other string in the component
 * went through react-i18next. Screen-reader users on sk/cs/de/hu/pl locales
 * got untranslated English for the most important data point on the chart (a
 * sentiment drop). The label now resolves `aria.sentimentSpike` (present in
 * all six locale bundles) with a `{{date}}` interpolation.
 *
 * Presentational component — i18n is the real English bundle from the shared
 * test setup (`src/test/setup.ts`).
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { SentimentTrend } from '../types';
import { SentimentTrendChart } from './SentimentTrendChart';

function trend(date: string, avgSentiment: number): SentimentTrend {
  return {
    id: `trend-${date}`,
    organizationId: 'org-1',
    buildingId: null,
    date,
    avgSentiment,
    messageCount: 10,
    negativeCount: 0,
    neutralCount: 0,
    positiveCount: 0,
    createdAt: `${date}T00:00:00Z`,
    updatedAt: `${date}T00:00:00Z`,
  };
}

describe('SentimentTrendChart spike marker a11y label', () => {
  it('labels a sentiment-drop spike marker through i18n, not a hardcoded English string', () => {
    // A >0.2 drop between two consecutive days triggers a spike marker.
    const trends: SentimentTrend[] = [trend('2026-01-01', 0.8), trend('2026-01-02', 0.5)];

    render(<SentimentTrendChart trends={trends} isLoading={false} />);

    // The translated label (en bundle: "Sentiment drop on {{date}}") is used…
    expect(screen.getByLabelText('Sentiment drop on 2026-01-02')).toBeInTheDocument();
    // …and the old hardcoded "Spike on <date>" string is gone.
    expect(screen.queryByLabelText('Spike on 2026-01-02')).toBeNull();
  });
});
