/**
 * FormRenderer Component
 *
 * Renders a form for user submission (Epic 54, Story 54.3).
 */

import type { FormField, SubmitFormRequest } from '@ppt/api-client';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface FormRendererProps {
  fields: FormField[];
  requireSignature?: boolean;
  isSubmitting?: boolean;
  onSubmit: (data: SubmitFormRequest) => void;
  onCancel: () => void;
}

export function FormRenderer({
  fields,
  requireSignature,
  isSubmitting,
  onSubmit,
  onCancel,
}: FormRendererProps) {
  const { t } = useTranslation();
  const [formData, setFormData] = useState<Record<string, unknown>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [signatureData, setSignatureData] = useState<string | undefined>();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const isDrawing = useRef(false);

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    for (const field of fields) {
      if (field.required) {
        const value = formData[field.id];
        if (value === undefined || value === null || value === '') {
          newErrors[field.id] = t('forms.renderer.requiredField');
        }
      }

      // Email validation
      if (field.fieldType === 'email' && formData[field.id]) {
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        if (!emailRegex.test(formData[field.id] as string)) {
          newErrors[field.id] = t('forms.renderer.invalidEmail');
        }
      }

      // Phone validation
      if (field.fieldType === 'phone' && formData[field.id]) {
        const phoneRegex = /^[+]?[\d\s-()]{7,}$/;
        if (!phoneRegex.test(formData[field.id] as string)) {
          newErrors[field.id] = t('forms.renderer.invalidPhone');
        }
      }
    }

    if (requireSignature && !signatureData) {
      newErrors._signature = t('forms.renderer.signatureRequiredError');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validateForm()) {
      onSubmit({
        data: formData,
        signatureData,
      });
    }
  };

  const handleInputChange = (fieldId: string, value: unknown) => {
    setFormData((prev) => ({ ...prev, [fieldId]: value }));
    // Clear error when user types
    if (errors[fieldId]) {
      setErrors((prev) => {
        const newErrors = { ...prev };
        delete newErrors[fieldId];
        return newErrors;
      });
    }
  };

  // Signature canvas handlers
  const startDrawing = (e: React.MouseEvent<HTMLCanvasElement>) => {
    isDrawing.current = true;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const rect = canvas.getBoundingClientRect();
    ctx.beginPath();
    ctx.moveTo(e.clientX - rect.left, e.clientY - rect.top);
  };

  const draw = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!isDrawing.current) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const rect = canvas.getBoundingClientRect();
    ctx.lineTo(e.clientX - rect.left, e.clientY - rect.top);
    ctx.stroke();
  };

  const stopDrawing = () => {
    isDrawing.current = false;
    const canvas = canvasRef.current;
    if (canvas) {
      setSignatureData(canvas.toDataURL());
    }
  };

  const clearSignature = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    setSignatureData(undefined);
  };

  const renderField = (field: FormField) => {
    const value = formData[field.id] ?? field.defaultValue ?? '';
    const error = errors[field.id];

    const baseInputClass = `w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${
      error ? 'border-red-500' : 'border-gray-300'
    }`;

    switch (field.fieldType) {
      case 'text':
      case 'email':
      case 'phone':
        return (
          <input
            type={
              field.fieldType === 'email' ? 'email' : field.fieldType === 'phone' ? 'tel' : 'text'
            }
            value={value as string}
            onChange={(e) => handleInputChange(field.id, e.target.value)}
            placeholder={field.placeholder}
            className={baseInputClass}
          />
        );

      case 'textarea':
        return (
          <textarea
            value={value as string}
            onChange={(e) => handleInputChange(field.id, e.target.value)}
            placeholder={field.placeholder}
            rows={4}
            className={baseInputClass}
          />
        );

      case 'number':
        return (
          <input
            type="number"
            value={value as number}
            onChange={(e) => handleInputChange(field.id, Number.parseFloat(e.target.value))}
            placeholder={field.placeholder}
            className={baseInputClass}
          />
        );

      case 'date':
        return (
          <input
            type="date"
            value={value as string}
            onChange={(e) => handleInputChange(field.id, e.target.value)}
            className={baseInputClass}
          />
        );

      case 'datetime':
        return (
          <input
            type="datetime-local"
            value={value as string}
            onChange={(e) => handleInputChange(field.id, e.target.value)}
            className={baseInputClass}
          />
        );

      case 'checkbox':
        return (
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id={field.id}
              checked={value as boolean}
              onChange={(e) => handleInputChange(field.id, e.target.checked)}
              className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
            />
            <label htmlFor={field.id} className="text-sm text-gray-700">
              {field.helpText || t('forms.renderer.yes')}
            </label>
          </div>
        );

      case 'radio':
        return (
          <div className="space-y-2">
            {field.options?.map((option) => (
              <div key={option.value} className="flex items-center gap-2">
                <input
                  type="radio"
                  id={`${field.id}-${option.value}`}
                  name={field.id}
                  value={option.value}
                  checked={value === option.value}
                  onChange={(e) => handleInputChange(field.id, e.target.value)}
                  className="border-gray-300 text-blue-600 focus:ring-blue-500"
                />
                <label htmlFor={`${field.id}-${option.value}`} className="text-sm text-gray-700">
                  {option.label}
                </label>
              </div>
            ))}
          </div>
        );

      case 'select':
        return (
          <select
            value={value as string}
            onChange={(e) => handleInputChange(field.id, e.target.value)}
            className={baseInputClass}
          >
            <option value="">{field.placeholder || t('forms.renderer.selectOption')}</option>
            {field.options?.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        );

      case 'multiselect':
        return (
          <select
            multiple
            value={value as string[]}
            onChange={(e) => {
              const selected = Array.from(e.target.selectedOptions, (opt) => opt.value);
              handleInputChange(field.id, selected);
            }}
            className={`${baseInputClass} h-32`}
          >
            {field.options?.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        );

      case 'file':
        return (
          <input
            type="file"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) {
                // Store file info
                handleInputChange(field.id, {
                  name: file.name,
                  size: file.size,
                  type: file.type,
                });
              }
            }}
            className="w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-medium file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
          />
        );

      case 'signature':
        return (
          <div className="space-y-2">
            <canvas
              ref={canvasRef}
              width={400}
              height={150}
              onMouseDown={startDrawing}
              onMouseMove={draw}
              onMouseUp={stopDrawing}
              onMouseLeave={stopDrawing}
              className={`border rounded-md bg-white cursor-crosshair ${
                error ? 'border-red-500' : 'border-gray-300'
              }`}
            />
            <button
              type="button"
              onClick={clearSignature}
              className="text-sm text-gray-500 hover:text-gray-700"
            >
              {t('forms.actions.clearSignature')}
            </button>
          </div>
        );

      default:
        return null;
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {fields.map((field) => (
        <fieldset key={field.id} className="border-0 p-0 m-0">
          <legend className="block text-sm font-medium text-gray-700 mb-1">
            {field.label}
            {field.required && <span className="text-red-500 ml-1">*</span>}
          </legend>
          {field.helpText && field.fieldType !== 'checkbox' && (
            <p className="text-sm text-gray-500 mb-2">{field.helpText}</p>
          )}
          {renderField(field)}
          {errors[field.id] && <p className="mt-1 text-sm text-red-600">{errors[field.id]}</p>}
        </fieldset>
      ))}

      {requireSignature && (
        <fieldset className="border-0 p-0 m-0">
          <legend className="block text-sm font-medium text-gray-700 mb-1">
            {t('forms.renderer.signatureLegend')}
            <span className="text-red-500 ml-1">*</span>
          </legend>
          <p className="text-sm text-gray-500 mb-2">{t('forms.renderer.signaturePrompt')}</p>
          <canvas
            ref={canvasRef}
            width={400}
            height={150}
            onMouseDown={startDrawing}
            onMouseMove={draw}
            onMouseUp={stopDrawing}
            onMouseLeave={stopDrawing}
            className={`border rounded-md bg-white cursor-crosshair ${
              errors._signature ? 'border-red-500' : 'border-gray-300'
            }`}
          />
          <button
            type="button"
            onClick={clearSignature}
            className="mt-2 text-sm text-gray-500 hover:text-gray-700"
          >
            {t('forms.actions.clearSignature')}
          </button>
          {errors._signature && <p className="mt-1 text-sm text-red-600">{errors._signature}</p>}
        </fieldset>
      )}

      <div className="flex gap-4 pt-4">
        <button
          type="submit"
          disabled={isSubmitting}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSubmitting ? t('forms.renderer.submitting') : t('forms.renderer.submitForm')}
        </button>
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 disabled:opacity-50"
        >
          {t('common.cancel')}
        </button>
      </div>
    </form>
  );
}
