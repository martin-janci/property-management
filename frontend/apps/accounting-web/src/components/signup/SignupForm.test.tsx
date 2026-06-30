/**
 * SignupForm client-side validation — accounting-web (#1828 finding-3).
 *
 * The signup POST is a deliberate no-op for now, but the client-side validation
 * (required fields, email shape, password length) is real behaviour and was
 * untested. The next-intl mock returns message keys verbatim, so the assertions
 * pin the validation branches by their stable keys.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SignupForm } from './SignupForm';

function fill(container: HTMLElement, name: string, value: string) {
  const input = container.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (!input) throw new Error(`input[name="${name}"] not found`);
  fireEvent.change(input, { target: { value } });
}

function submit(container: HTMLElement) {
  const form = container.querySelector('form');
  if (!form) throw new Error('form not found');
  fireEvent.submit(form);
}

describe('SignupForm validation', () => {
  it('reports required fields when submitted empty', () => {
    const { container } = render(<SignupForm />);
    submit(container);

    expect(screen.getByText('validation.companyRequired')).toBeInTheDocument();
    expect(screen.getByText('validation.emailRequired')).toBeInTheDocument();
    expect(screen.getByText('validation.passwordShort')).toBeInTheDocument();
    // No "coming soon" success notice on a failed validation.
    expect(screen.queryByText('comingSoon')).not.toBeInTheDocument();
  });

  it('rejects a malformed email', () => {
    const { container } = render(<SignupForm />);
    fill(container, 'company', 'Acme s.r.o.');
    fill(container, 'email', 'not-an-email');
    fill(container, 'password', 'supersecret');
    submit(container);

    expect(screen.getByText('validation.emailInvalid')).toBeInTheDocument();
    expect(screen.queryByText('comingSoon')).not.toBeInTheDocument();
  });

  it('rejects a short password', () => {
    const { container } = render(<SignupForm />);
    fill(container, 'company', 'Acme s.r.o.');
    fill(container, 'email', 'owner@acme.test');
    fill(container, 'password', 'short');
    submit(container);

    expect(screen.getByText('validation.passwordShort')).toBeInTheDocument();
    expect(screen.queryByText('comingSoon')).not.toBeInTheDocument();
  });

  it('shows the coming-soon notice when all fields are valid', () => {
    const { container } = render(<SignupForm />);
    fill(container, 'company', 'Acme s.r.o.');
    fill(container, 'email', 'owner@acme.test');
    fill(container, 'password', 'supersecret');
    submit(container);

    expect(screen.getByText('comingSoon')).toBeInTheDocument();
    expect(screen.queryByText('validation.emailInvalid')).not.toBeInTheDocument();
    expect(screen.queryByText('validation.passwordShort')).not.toBeInTheDocument();
  });
});
