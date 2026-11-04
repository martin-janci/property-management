/**
 * Tabs between the accounting screens (invoices ↔ payment matching), so
 * neither page is a dead end / orphan.
 */
import { useTranslation } from 'react-i18next';
import { NavLink } from 'react-router-dom';

const TABS = [
  { to: '/accounting/invoices', key: 'accounting.nav.invoices' },
  { to: '/accounting/payment-matching', key: 'accounting.nav.paymentMatching' },
] as const;

export function AccountingSubNav() {
  const { t } = useTranslation();
  return (
    <nav aria-label={t('accounting.nav.label')} className="mb-6 border-b border-gray-200">
      <ul className="flex gap-6 -mb-px">
        {TABS.map((tab) => (
          <li key={tab.to}>
            <NavLink
              to={tab.to}
              className={({ isActive }) =>
                `inline-block pb-2 text-sm font-medium border-b-2 ${
                  isActive
                    ? 'border-blue-600 text-blue-700'
                    : 'border-transparent text-gray-500 hover:text-gray-700'
                }`
              }
            >
              {t(tab.key)}
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  );
}
