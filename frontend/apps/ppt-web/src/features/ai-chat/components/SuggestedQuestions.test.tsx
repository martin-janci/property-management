/**
 * SuggestedQuestions Component Tests
 * Epic 127: AI Chatbot Interface
 */

/// <reference types="vitest/globals" />
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { SuggestedQuestion } from '../types';
import { getDefaultSuggestedQuestions, SuggestedQuestions } from './SuggestedQuestions';

// Mock i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'aiChat.welcomeTitle': 'How can I help you today?',
        'aiChat.welcomeDescription':
          'I can answer questions about your building, help with fault reporting, voting, and more.',
        'aiChat.suggestedQuestions': 'Try asking about',
      };
      return translations[key] || key;
    },
  }),
}));

const mockQuestions: SuggestedQuestion[] = [
  { id: '1', text: 'How do I report a fault?', category: 'Faults' },
  { id: '2', text: 'What are the upcoming votes?', category: 'Voting' },
  { id: '3', text: 'How do I submit meter readings?', category: 'Utilities' },
];

describe('SuggestedQuestions', () => {
  it('renders welcome message', () => {
    render(<SuggestedQuestions questions={mockQuestions} onSelect={vi.fn()} />);

    expect(screen.getByText('How can I help you today?')).toBeInTheDocument();
    expect(screen.getByText(/I can answer questions about your building/)).toBeInTheDocument();
  });

  it('renders all suggested questions', () => {
    render(<SuggestedQuestions questions={mockQuestions} onSelect={vi.fn()} />);

    expect(screen.getByText('How do I report a fault?')).toBeInTheDocument();
    expect(screen.getByText('What are the upcoming votes?')).toBeInTheDocument();
    expect(screen.getByText('How do I submit meter readings?')).toBeInTheDocument();
  });

  it('displays category labels', () => {
    render(<SuggestedQuestions questions={mockQuestions} onSelect={vi.fn()} />);

    expect(screen.getByText('Faults')).toBeInTheDocument();
    expect(screen.getByText('Voting')).toBeInTheDocument();
    expect(screen.getByText('Utilities')).toBeInTheDocument();
  });

  it('calls onSelect when a question is clicked', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<SuggestedQuestions questions={mockQuestions} onSelect={onSelect} />);

    await user.click(screen.getByText('How do I report a fault?'));

    expect(onSelect).toHaveBeenCalledWith('How do I report a fault?');
  });

  it('disables buttons when disabled prop is true', () => {
    render(<SuggestedQuestions questions={mockQuestions} onSelect={vi.fn()} disabled={true} />);

    const buttons = screen.getAllByRole('button');
    for (const button of buttons) {
      expect(button).toBeDisabled();
    }
  });

  it('does not call onSelect when disabled', async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<SuggestedQuestions questions={mockQuestions} onSelect={onSelect} disabled={true} />);

    await user.click(screen.getByText('How do I report a fault?'));

    expect(onSelect).not.toHaveBeenCalled();
  });

  it('renders nothing when questions array is empty', () => {
    const { container } = render(<SuggestedQuestions questions={[]} onSelect={vi.fn()} />);

    expect(container.firstChild).toBeNull();
  });
});

describe('getDefaultSuggestedQuestions', () => {
  it('returns an array of suggested questions', () => {
    const questions = getDefaultSuggestedQuestions();

    expect(Array.isArray(questions)).toBe(true);
    expect(questions.length).toBeGreaterThan(0);
  });

  it('returns questions with required properties', () => {
    const questions = getDefaultSuggestedQuestions();

    for (const question of questions) {
      expect(question).toHaveProperty('id');
      expect(question).toHaveProperty('text');
      expect(question).toHaveProperty('category');
    }
  });

  it('returns unique question IDs', () => {
    const questions = getDefaultSuggestedQuestions();
    const ids = questions.map((q) => q.id);
    const uniqueIds = new Set(ids);

    expect(uniqueIds.size).toBe(ids.length);
  });
});
