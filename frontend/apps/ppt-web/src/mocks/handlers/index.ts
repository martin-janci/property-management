import { buildingsHandlers } from './buildings';
import { reportsHandlers } from './reports';

export const handlers = [...buildingsHandlers, ...reportsHandlers];
