/**
 * Sensor status / alert-severity pill (Epic 14 — FR71/FR74).
 */
import type { JSX } from 'react';

const STATUS_STYLES: Record<string, string> = {
  active: 'bg-green-100 text-green-800',
  online: 'bg-green-100 text-green-800',
  offline: 'bg-gray-200 text-gray-700',
  error: 'bg-red-100 text-red-800',
  warning: 'bg-amber-100 text-amber-800',
  critical: 'bg-red-100 text-red-800',
  info: 'bg-blue-100 text-blue-800',
};

export function SensorStatusBadge({ value }: { value: string }): JSX.Element {
  const cls = STATUS_STYLES[value?.toLowerCase()] ?? 'bg-gray-100 text-gray-700';
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${cls}`}
    >
      {value}
    </span>
  );
}
