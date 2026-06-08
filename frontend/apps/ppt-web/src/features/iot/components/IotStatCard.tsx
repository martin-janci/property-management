/**
 * Single KPI tile for the IoT dashboard rollup (Epic 14 — FR75).
 */
import type { JSX } from 'react';

export interface IotStatCardProps {
  label: string;
  value: number | string;
  tone?: 'default' | 'success' | 'warning' | 'danger';
}

const TONE_STYLES: Record<NonNullable<IotStatCardProps['tone']>, string> = {
  default: 'text-gray-900',
  success: 'text-green-600',
  warning: 'text-amber-600',
  danger: 'text-red-600',
};

export function IotStatCard({ label, value, tone = 'default' }: IotStatCardProps): JSX.Element {
  return (
    <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
      <div className="text-sm font-medium text-gray-500">{label}</div>
      <div className={`mt-1 text-2xl font-semibold ${TONE_STYLES[tone]}`}>{value}</div>
    </div>
  );
}
