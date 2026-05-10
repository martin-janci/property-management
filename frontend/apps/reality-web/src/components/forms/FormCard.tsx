/**
 * FormCard + FieldGroupTitle + FormActions — layout primitives for the
 * modern form system. Mirrors the design's `.form-card`,
 * `.field-group-title`, and `.form-actions` rules from pages/sell.html.
 */

import type { ReactNode } from 'react';
import './forms.css';

export function FormCard({
  title,
  subtitle,
  step,
  children,
  className,
}: {
  /** Section title rendered as h2 inside the card. */
  title?: ReactNode;
  /** Optional subtitle shown under the title. */
  subtitle?: ReactNode;
  /** Optional step pill (e.g. "Krok 1 / 5") shown next to the title. */
  step?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={['form-card', className].filter(Boolean).join(' ')}>
      {title && (
        <h2 className="form-card-title">
          {title}
          {step && <span className="form-card-step-pill">{step}</span>}
        </h2>
      )}
      {subtitle && <p className="form-card-sub">{subtitle}</p>}
      {children}
    </section>
  );
}

export function FieldGroupTitle({ children }: { children: ReactNode }) {
  return <div className="field-group-title">{children}</div>;
}

export function FormActions({
  left,
  children,
  className,
}: {
  /** Left-hand content (typically save-state meta like "Saved 2s ago"). */
  left?: ReactNode;
  /** Buttons / actions on the right. */
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={['form-actions', className].filter(Boolean).join(' ')}>
      <div className="form-actions-left">{left}</div>
      <div className="form-actions-right">{children}</div>
    </div>
  );
}
