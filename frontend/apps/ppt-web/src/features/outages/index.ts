/**
 * Outages feature module barrel export.
 * UC-12: Utility Outages
 */

export type {
  OutageCommodity,
  OutageFormData,
  OutageSeverity,
  OutageStatus,
  OutageSummary,
} from './components';
// Components
export {
  OutageCard,
  OutageForm,
  OutageList,
} from './components';
export type { ListOutagesParams, OutageDetail } from './pages';
// Pages
export {
  CreateOutagePage,
  EditOutagePage,
  OutagesPage,
  ViewOutagePage,
} from './pages';
