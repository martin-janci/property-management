/// <reference types="vitest/globals" />
import { describe, expect, it } from 'vitest';
import { validateBuilding, valuesFromBuilding } from './BuildingFormPage';

describe('BuildingFormPage validation', () => {
  it('requires address fields and positive whole floors / entrances', () => {
    const v = { ...valuesFromBuilding(), floorCount: '0', totalEntrances: '1.5' };
    expect(validateBuilding(v)).toEqual({
      street: 'streetRequired',
      city: 'cityRequired',
      postalCode: 'postalCodeRequired',
      floorCount: 'floorsInvalid',
      totalEntrances: 'entrancesInvalid',
    });
  });

  it('accepts a complete building and rejects an implausible year', () => {
    const v = {
      ...valuesFromBuilding(),
      street: 'Hlavná 1',
      city: 'Bratislava',
      postalCode: '81101',
    };
    expect(validateBuilding(v)).toEqual({});
    expect(validateBuilding({ ...v, yearBuilt: '12' })).toEqual({ yearBuilt: 'yearInvalid' });
  });

  it('defaults new buildings to SK with one floor and one entrance', () => {
    expect(valuesFromBuilding()).toMatchObject({
      country: 'SK',
      floorCount: '1',
      totalEntrances: '1',
    });
  });
});
