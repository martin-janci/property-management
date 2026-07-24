/**
 * Wiring test for RegisterPage's signup-funnel instrumentation (issue #2530).
 *
 * Asserts the registration-submit leg of the funnel emits
 * `signup.registration_submitted` — and only after the server accepts the
 * registration (never on a validation failure or a rejected request).
 */
/// <reference types="vitest/globals" />
import { fireEvent, render, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  __clearAnalyticsSinks,
  type AnalyticsEvent,
  registerAnalyticsSink,
} from '../../../lib/analytics';
import { SIGNUP_EVENTS } from '../analytics';
import { RegisterPage } from './RegisterPage';

const registerFn = vi.fn();

vi.mock('../authApiClient', () => ({
  getAuthApi: () => ({ register: registerFn }),
}));

let captured: AnalyticsEvent[];

function fillValidForm(container: HTMLElement) {
  const set = (id: string, value: string) => {
    const el = container.querySelector<HTMLInputElement>(`#${id}`);
    if (!el) throw new Error(`missing input #${id}`);
    fireEvent.change(el, { target: { value } });
  };
  set('firstName', 'Ada');
  set('lastName', 'Lovelace');
  set('email', 'ada@example.com');
  set('password', 'supersecret');
  set('confirmPassword', 'supersecret');
}

beforeEach(() => {
  captured = [];
  registerFn.mockReset();
  registerAnalyticsSink((e) => captured.push(e));
});

afterEach(() => {
  __clearAnalyticsSinks();
});

describe('RegisterPage signup-funnel wiring', () => {
  function renderPage() {
    return render(
      <MemoryRouter>
        <RegisterPage />
      </MemoryRouter>
    );
  }

  it('emits registration_submitted after a successful register call', async () => {
    registerFn.mockResolvedValue({});
    const { container } = renderPage();
    fillValidForm(container);
    fireEvent.submit(container.querySelector('form') as HTMLFormElement);

    await waitFor(() => {
      expect(registerFn).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(captured.map((e) => e.event)).toEqual([SIGNUP_EVENTS.registrationSubmitted]);
    });
    expect(captured[0].properties).toMatchObject({ method: 'email_password' });
  });

  it('does not emit when the register request rejects', async () => {
    registerFn.mockRejectedValue(new Error('EMAIL_ALREADY_EXISTS'));
    const { container } = renderPage();
    fillValidForm(container);
    fireEvent.submit(container.querySelector('form') as HTMLFormElement);

    await waitFor(() => {
      expect(registerFn).toHaveBeenCalledTimes(1);
    });
    expect(captured).toHaveLength(0);
  });

  it('does not emit (or call the API) when client-side validation fails', async () => {
    registerFn.mockResolvedValue({});
    const { container } = renderPage();
    // Leave fields empty → validation blocks the submit.
    fireEvent.submit(container.querySelector('form') as HTMLFormElement);

    await Promise.resolve();
    expect(registerFn).not.toHaveBeenCalled();
    expect(captured).toHaveLength(0);
  });
});
