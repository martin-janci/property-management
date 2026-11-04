/**
 * InvoiceStatusChart component for visualizing invoice distribution.
 */

import { useTranslation } from 'react-i18next';
import { invoiceStatusKey } from '../utils/constants';

interface InvoiceStatusChartProps {
  data: {
    draft: number;
    sent: number;
    overdue: number;
    paid: number;
  };
}

const STATUS_COLORS: Record<string, string> = {
  draft: 'bg-gray-400',
  sent: 'bg-blue-500',
  overdue: 'bg-red-500',
  paid: 'bg-green-500',
};

export function InvoiceStatusChart({ data }: InvoiceStatusChartProps) {
  const { t, i18n } = useTranslation();
  const total = data.draft + data.sent + data.overdue + data.paid;

  if (total === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">
          {t('financial.invoiceChart.title')}
        </h3>
        <p className="text-gray-500 text-center py-8">{t('financial.invoiceChart.empty')}</p>
      </div>
    );
  }

  const getPercentage = (value: number) => {
    const percent = total > 0 ? (value / total) * 100 : 0;
    return percent.toLocaleString(i18n.language, {
      minimumFractionDigits: total > 0 ? 1 : 0,
      maximumFractionDigits: 1,
    });
  };

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h3 className="text-lg font-medium text-gray-900 mb-4">
        {t('financial.invoiceChart.title')}
      </h3>

      {/* Horizontal stacked bar */}
      <div className="h-8 flex rounded-lg overflow-hidden mb-6">
        {Object.entries(data).map(([status, value]) => {
          const percentage = (value / total) * 100;
          if (percentage === 0) return null;
          return (
            <div
              key={status}
              className={`${STATUS_COLORS[status]} transition-all duration-300`}
              style={{ width: `${percentage}%` }}
              title={`${t(invoiceStatusKey(status))}: ${value}`}
            />
          );
        })}
      </div>

      {/* Legend */}
      <div className="grid grid-cols-2 gap-4">
        {Object.entries(data).map(([status, value]) => (
          <div key={status} className="flex items-center justify-between">
            <div className="flex items-center">
              <span className={`w-3 h-3 rounded-full ${STATUS_COLORS[status]} mr-2`} />
              <span className="text-sm text-gray-600">{t(invoiceStatusKey(status))}</span>
            </div>
            <div className="text-right">
              <span className="text-sm font-medium text-gray-900">{value}</span>
              <span className="text-sm text-gray-500 ml-1">({getPercentage(value)}%)</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
