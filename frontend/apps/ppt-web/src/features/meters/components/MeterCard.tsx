/**
 * MeterCard component - displays a meter summary card.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useTranslation } from 'react-i18next';
import type { Meter, MeterType, MeterUnit } from '../types';

interface MeterCardProps {
  meter: Meter;
  onView?: (id: string) => void;
  onSubmitReading?: (id: string) => void;
}

const meterTypeIcons: Record<MeterType, string> = {
  electricity: 'bolt',
  gas: 'fire',
  water: 'droplet',
  heat: 'thermometer',
  cold_water: 'droplet',
  hot_water: 'droplet',
};

const meterTypeColors: Record<MeterType, string> = {
  electricity: 'bg-yellow-100 text-yellow-800',
  gas: 'bg-orange-100 text-orange-800',
  water: 'bg-blue-100 text-blue-800',
  heat: 'bg-red-100 text-red-800',
  cold_water: 'bg-cyan-100 text-cyan-800',
  hot_water: 'bg-rose-100 text-rose-800',
};

function formatValue(value: number, unit: MeterUnit): string {
  return `${value.toLocaleString(undefined, { maximumFractionDigits: 2 })} ${unit}`;
}

function MeterTypeIcon({ type }: { type: MeterType }) {
  const iconType = meterTypeIcons[type];

  if (iconType === 'bolt') {
    return (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
        <title>Electricity</title>
        <path
          fillRule="evenodd"
          d="M11.3 1.046A1 1 0 0112 2v5h4a1 1 0 01.82 1.573l-7 10A1 1 0 018 18v-5H4a1 1 0 01-.82-1.573l7-10a1 1 0 011.12-.38z"
          clipRule="evenodd"
        />
      </svg>
    );
  }

  if (iconType === 'fire') {
    return (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
        <title>Gas</title>
        <path
          fillRule="evenodd"
          d="M12.395 2.553a1 1 0 00-1.45-.385c-.345.23-.614.558-.822.88-.214.33-.403.713-.57 1.116-.334.804-.614 1.768-.84 2.734a31.365 31.365 0 00-.613 3.58 2.64 2.64 0 01-.945-1.067c-.328-.68-.398-1.534-.398-2.654A1 1 0 005.05 6.05 6.981 6.981 0 003 11a7 7 0 1011.95-4.95c-.592-.591-.98-.985-1.348-1.467-.363-.476-.724-1.063-1.207-2.03zM12.12 15.12A3 3 0 017 13s.879.5 2.5.5c0-1 .5-4 1.25-4.5.5 1 .786 1.293 1.371 1.879A2.99 2.99 0 0113 13a2.99 2.99 0 01-.879 2.121z"
          clipRule="evenodd"
        />
      </svg>
    );
  }

  if (iconType === 'droplet') {
    return (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
        <title>Water</title>
        <path
          fillRule="evenodd"
          d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z"
          clipRule="evenodd"
        />
      </svg>
    );
  }

  if (iconType === 'thermometer') {
    return (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
        <title>Heat</title>
        <path
          fillRule="evenodd"
          d="M10 2a1 1 0 011 1v1.323l3.954 1.582 1.599-.8a1 1 0 01.894 1.79l-1.233.616 1.738 5.42a1 1 0 01-.285 1.05A3.989 3.989 0 0115 15a3.989 3.989 0 01-2.667-1.019 1 1 0 01-.285-1.05l1.715-5.349L11 6.477V16h2a1 1 0 110 2H7a1 1 0 110-2h2V6.477L6.237 7.582l1.715 5.349a1 1 0 01-.285 1.05A3.989 3.989 0 015 15a3.989 3.989 0 01-2.667-1.019 1 1 0 01-.285-1.05l1.738-5.42-1.233-.617a1 1 0 01.894-1.788l1.599.799L9 4.323V3a1 1 0 011-1z"
          clipRule="evenodd"
        />
      </svg>
    );
  }

  return null;
}

export function MeterCard({ meter, onView, onSubmitReading }: MeterCardProps) {
  const { t, i18n } = useTranslation();

  const lastReadingFormatted =
    meter.lastReadingValue !== undefined
      ? formatValue(meter.lastReadingValue, meter.unit)
      : t('meters.noReadings');

  const lastReadingDate = meter.lastReadingDate
    ? new Date(meter.lastReadingDate).toLocaleDateString(i18n.language)
    : null;

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <span className={`p-2 rounded-full ${meterTypeColors[meter.meterType]}`}>
              <MeterTypeIcon type={meter.meterType} />
            </span>
            <div>
              <h3 className="text-lg font-semibold text-gray-900">
                {t(`meters.types.${meter.meterType}`)}
              </h3>
              <p className="text-sm text-gray-500">{meter.serialNumber}</p>
            </div>
          </div>

          {meter.location && (
            <p className="mt-2 text-sm text-gray-600">
              <span className="font-medium">{t('meters.location')}:</span> {meter.location}
            </p>
          )}

          <div className="mt-3 p-3 bg-gray-50 rounded-lg">
            <p className="text-sm text-gray-500">{t('meters.lastReading')}</p>
            <p className="text-xl font-bold text-gray-900">{lastReadingFormatted}</p>
            {lastReadingDate && <p className="text-xs text-gray-400">{lastReadingDate}</p>}
          </div>

          {meter.unitDesignation && (
            <p className="mt-2 text-xs text-gray-500">
              {t('meters.unit')}: {meter.unitDesignation}
            </p>
          )}
        </div>
      </div>

      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        <button
          type="button"
          onClick={() => onView?.(meter.id)}
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.view')}
        </button>
        {meter.isActive && (
          <button
            type="button"
            onClick={() => onSubmitReading?.(meter.id)}
            className="text-sm text-green-600 hover:text-green-800"
          >
            {t('meters.submitReading')}
          </button>
        )}
      </div>
    </div>
  );
}
