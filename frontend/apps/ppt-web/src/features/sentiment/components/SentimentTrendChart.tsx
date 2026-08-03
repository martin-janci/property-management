/**
 * Trend line chart for daily average sentiment scores.
 * Uses a simple SVG sparkline — no chart library dependency.
 */
import type { JSX } from 'react';
import { useTranslation } from 'react-i18next';
import type { SentimentTrend } from '../types';

interface SentimentTrendChartProps {
  trends: SentimentTrend[];
  isLoading: boolean;
}

function SentimentLine({ trends }: { trends: SentimentTrend[] }): JSX.Element {
  const { t } = useTranslation();

  if (trends.length < 2) {
    return (
      <p className="py-8 text-center text-sm text-gray-400">
        {t('sentiment.notEnoughData', { defaultValue: 'Not enough data to draw trend.' })}
      </p>
    );
  }

  const W = 600;
  const H = 120;
  const PAD = 8;

  const sorted = [...trends].sort((a, b) => a.date.localeCompare(b.date));
  const scores = sorted.map((t) => t.avgSentiment);
  const minScore = Math.min(...scores);
  const maxScore = Math.max(...scores);
  const range = maxScore - minScore || 1;

  const xStep = (W - PAD * 2) / (sorted.length - 1);
  const toX = (i: number) => PAD + i * xStep;
  const toY = (v: number) => H - PAD - ((v - minScore) / range) * (H - PAD * 2);

  const d = sorted
    .map((t, i) => `${i === 0 ? 'M' : 'L'}${toX(i).toFixed(1)},${toY(t.avgSentiment).toFixed(1)}`)
    .join(' ');

  // Highlight spikes (drops > 0.2)
  const spikes = sorted
    .slice(1)
    .map((t, i) => ({ t, i: i + 1, delta: t.avgSentiment - sorted[i].avgSentiment }))
    .filter((s) => s.delta < -0.2);

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="w-full" aria-label={t('aria.sentimentTrend')}>
      {/* Zero line at 0.5 */}
      <line
        x1={PAD}
        y1={toY(0.5).toFixed(1)}
        x2={W - PAD}
        y2={toY(0.5).toFixed(1)}
        stroke="#e5e7eb"
        strokeDasharray="4"
      />
      <path d={d} fill="none" stroke="#6366f1" strokeWidth="2" strokeLinejoin="round" />
      {/* Spike markers */}
      {spikes.map((s) => (
        <circle
          key={s.i}
          cx={toX(s.i).toFixed(1)}
          cy={toY(s.t.avgSentiment).toFixed(1)}
          r="4"
          fill="#ef4444"
          aria-label={`Spike on ${s.t.date}`}
        />
      ))}
      {/* Labels */}
      {[sorted[0], sorted[sorted.length - 1]].map((t, idx) => (
        <text
          key={idx}
          x={idx === 0 ? PAD : W - PAD}
          y={H}
          fontSize="10"
          textAnchor={idx === 0 ? 'start' : 'end'}
          fill="#9ca3af"
        >
          {t.date}
        </text>
      ))}
    </svg>
  );
}

export function SentimentTrendChart({ trends, isLoading }: SentimentTrendChartProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border bg-white p-4 shadow-sm">
      <h2 className="mb-3 text-base font-semibold text-gray-900">
        {t('sentiment.trendTitle', { defaultValue: 'Sentiment Trend (30 days)' })}
      </h2>
      {isLoading ? (
        <div className="h-32 animate-pulse rounded bg-gray-100" />
      ) : (
        <SentimentLine trends={trends} />
      )}
    </div>
  );
}
