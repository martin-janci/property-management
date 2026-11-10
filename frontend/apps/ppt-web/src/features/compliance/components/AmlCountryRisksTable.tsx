/**
 * AML Country Risks Table (Epic 67, Story 67.1).
 *
 * Presentational table of per-country AML risk ratings.
 * Extracted from AmlDashboardPage to keep the page a thin orchestrator.
 */

import type React from 'react';
import { useTranslation } from 'react-i18next';

export interface CountryRiskDisplay {
  country_code: string;
  country_name: string;
  risk_rating: string;
  is_sanctioned: boolean;
  fatf_status?: string;
}

export interface AmlCountryRisksTableProps {
  countryRisks: CountryRiskDisplay[];
}

export const AmlCountryRisksTable: React.FC<AmlCountryRisksTableProps> = ({ countryRisks }) => {
  const { t } = useTranslation();

  return (
    <div className="aml-country-risks-section">
      <h2>{t('aml.countryRisks.title')}</h2>
      <table className="aml-country-risks-table">
        <thead>
          <tr>
            <th>{t('aml.countryRisks.country')}</th>
            <th>{t('aml.countryRisks.riskRating')}</th>
            <th>{t('aml.countryRisks.sanctioned')}</th>
            <th>{t('aml.countryRisks.fatfStatus')}</th>
          </tr>
        </thead>
        <tbody>
          {countryRisks.map((country) => (
            <tr key={country.country_code} className={`risk-${country.risk_rating}`}>
              <td>
                {country.country_code} - {country.country_name}
              </td>
              <td>
                <span className={`risk-badge ${country.risk_rating}`}>
                  {country.risk_rating.toUpperCase()}
                </span>
              </td>
              <td>{country.is_sanctioned ? t('common.yes') : t('common.no')}</td>
              <td>{country.fatf_status || '-'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

AmlCountryRisksTable.displayName = 'AmlCountryRisksTable';
