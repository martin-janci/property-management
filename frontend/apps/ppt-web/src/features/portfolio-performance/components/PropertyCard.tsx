// Epic 144: Portfolio Performance Analytics - Property Card Component
import type React from 'react';

interface PropertyCardProps {
  id: string;
  propertyName: string;
  buildingAddress?: string;
  currentValue: number;
  equity: number;
  ltv?: number;
  noi: number;
  capRate?: number;
  cashOnCash?: number;
  dscr?: number;
  occupancyRate?: number;
  monthlyCashFlow?: number;
  performanceStatus: string;
  currency: string;
  onClick?: () => void;
}

export const PropertyCard: React.FC<PropertyCardProps> = ({
  propertyName,
  buildingAddress,
  currentValue,
  equity,
  ltv,
  noi,
  capRate,
  cashOnCash,
  dscr,
  monthlyCashFlow,
  performanceStatus,
  currency,
  onClick,
}) => {
  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'excellent':
        return 'bg-green-100 text-green-800';
      case 'good':
        return 'bg-blue-100 text-blue-800';
      case 'fair':
        return 'bg-yellow-100 text-yellow-800';
      case 'needs_attention':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getStatusLabel = (status: string) => {
    switch (status) {
      case 'excellent':
        return 'Excellent';
      case 'good':
        return 'Good';
      case 'fair':
        return 'Fair';
      case 'needs_attention':
        return 'Needs Attention';
      default:
        return 'No Data';
    }
  };

  const interactive = typeof onClick === 'function';

  return (
    <div
      className={`bg-white rounded-lg shadow-md p-6 transition-shadow ${
        interactive ? 'cursor-pointer hover:shadow-lg' : ''
      }`}
      onClick={onClick}
    >
      <div className="flex justify-between items-start mb-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">{propertyName}</h3>
          {buildingAddress && <p className="text-sm text-gray-500 mt-1">{buildingAddress}</p>}
        </div>
        <span className={`px-2 py-1 text-xs rounded-full ${getStatusColor(performanceStatus)}`}>
          {getStatusLabel(performanceStatus)}
        </span>
      </div>

      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <p className="text-sm text-gray-500">Current Value</p>
          <p className="text-xl font-bold text-gray-900">{formatCurrency(currentValue)}</p>
        </div>
        <div>
          <p className="text-sm text-gray-500">Equity</p>
          <p className="text-xl font-bold text-green-600">{formatCurrency(equity)}</p>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-3 pt-4 border-t border-gray-100">
        <div>
          <p className="text-xs text-gray-500">NOI</p>
          <p className="text-sm font-semibold">{formatCurrency(noi)}</p>
        </div>
        {capRate !== undefined && (
          <div>
            <p className="text-xs text-gray-500">Cap Rate</p>
            <p className="text-sm font-semibold">{capRate.toFixed(2)}%</p>
          </div>
        )}
        {cashOnCash !== undefined && (
          <div>
            <p className="text-xs text-gray-500">Cash-on-Cash</p>
            <p className="text-sm font-semibold">{cashOnCash.toFixed(2)}%</p>
          </div>
        )}
        {dscr !== undefined && (
          <div>
            <p className="text-xs text-gray-500">DSCR</p>
            <p className="text-sm font-semibold">{dscr.toFixed(2)}x</p>
          </div>
        )}
      </div>

      {(ltv !== undefined || monthlyCashFlow !== undefined) && (
        <div className="grid grid-cols-2 gap-4 pt-3 mt-3 border-t border-gray-100">
          {ltv !== undefined && (
            <div>
              <p className="text-xs text-gray-500">LTV</p>
              <p className="text-sm font-semibold">{ltv.toFixed(1)}%</p>
            </div>
          )}
          {monthlyCashFlow !== undefined && (
            <div>
              <p className="text-xs text-gray-500">Monthly Cash Flow</p>
              <p
                className={`text-sm font-semibold ${monthlyCashFlow >= 0 ? 'text-green-600' : 'text-red-600'}`}
              >
                {formatCurrency(monthlyCashFlow)}
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
