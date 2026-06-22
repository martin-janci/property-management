import type { JSX } from 'react';

type Tone = 'default' | 'success' | 'warning' | 'danger';

const toneClasses: Record<Tone, string> = {
  default: 'bg-white text-gray-900',
  success: 'bg-green-50 text-green-800',
  warning: 'bg-yellow-50 text-yellow-800',
  danger: 'bg-red-50 text-red-800',
};

interface SentimentStatCardProps {
  label: string;
  value: string | number;
  tone?: Tone;
}

export function SentimentStatCard({
  label,
  value,
  tone = 'default',
}: SentimentStatCardProps): JSX.Element {
  return (
    <div className={`rounded-lg border p-4 shadow-sm ${toneClasses[tone]}`}>
      <p className="text-sm font-medium opacity-70">{label}</p>
      <p className="mt-1 text-2xl font-semibold">{value}</p>
    </div>
  );
}
