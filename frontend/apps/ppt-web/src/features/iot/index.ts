/**
 * IoT / Smart-Building feature (Epic 14 — FR71-75).
 *
 * Manager-web front door for the IoT sensor backend.
 */
export * from './components';
export {
  type AlertStateFilter,
  IotAlertsPage,
  type IotAlertsPageProps,
  IotDashboardPage,
  type IotDashboardPageProps,
  IotThresholdConfigPage,
  type IotThresholdConfigPageProps,
  type SensorFormBuildingOption,
  SensorFormPage,
  type SensorFormPageProps,
  type SensorFormValues,
  SensorListPage,
  type SensorListPageProps,
} from './pages';
