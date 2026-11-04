/**
 * Sensor connection type is a select of the values the database accepts
 * (round-3 audit #12). A free-text field let managers type "http" or
 * "modbus", which the `sensors.connection_type` CHECK rejected with a 500.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import i18n from 'i18next';
import { afterEach, describe, expect, it, vi } from 'vitest';
import sk from '../../../../messages/sk.json';
import { CONNECTION_TYPE_OPTIONS, SensorFormPage } from './SensorFormPage';

function renderForm(onSubmit = vi.fn()) {
  const utils = render(
    <SensorFormPage
      mode="create"
      buildings={[{ id: 'b1', name: 'Building 1' }]}
      initialValues={{ buildingId: 'b1', name: 'Hall sensor', sensorType: 'temperature' }}
      isSaving={false}
      onSubmit={onSubmit}
      onCancel={() => {}}
    />
  );
  return { ...utils, onSubmit };
}

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('SensorFormPage connection type', () => {
  it('offers only api, mqtt, webhook and polling (plus "not specified")', () => {
    const { container } = renderForm();
    const select = container.querySelector('#sensor-connection') as HTMLSelectElement;
    expect(select.tagName).toBe('SELECT');
    expect(Array.from(select.options).map((o) => o.value)).toEqual([
      '',
      ...CONNECTION_TYPE_OPTIONS,
    ]);
  });

  it('submits the selected value', () => {
    const { container, onSubmit } = renderForm();
    const select = container.querySelector('#sensor-connection') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: 'mqtt' } });
    fireEvent.submit(container.querySelector('form') as HTMLFormElement);
    expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ connectionType: 'mqtt' }));
  });

  it('labels the options in Slovak', async () => {
    i18n.addResourceBundle('sk', 'translation', sk, true, true);
    await i18n.changeLanguage('sk');
    renderForm();
    expect(screen.getByRole('option', { name: 'Neuvedené' })).toBeTruthy();
    expect(screen.getByRole('option', { name: 'Pravidelné dopytovanie' })).toBeTruthy();
  });
});
