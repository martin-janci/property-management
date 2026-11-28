/**
 * Help center for contextual help, FAQs, and tutorials.
 *
 * Epic 50 - Story 50.2: Contextual Help
 * Epic 50 - Story 50.3: FAQ & Tutorials
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

import type { FAQCategory, FAQItem, HelpContent, Tutorial } from './types';

const FAQ_VOTES_KEY = '@ppt/faq_votes';

/**
 * Minimal translator shape (compatible with react-i18next's `t`). Passed in by
 * the UI layer so this non-React singleton stays decoupled from the i18n
 * instance — see the i18n note on the class.
 */
export type Translator = (key: string) => string;

/**
 * Provides help content, FAQs, and tutorials.
 *
 * i18n: all user-facing strings (FAQ questions/answers, category labels,
 * tutorial titles/descriptions/steps, contextual-help copy) live in the locale
 * resource files under the `help.*` namespace, NOT in this module. The display
 * fields of the returned `FAQItem` / `Tutorial` / `HelpContent` objects, and
 * the `labelKey` of `getFAQCategories()`, therefore hold **i18n keys** — the
 * rendering components resolve them with `t(key)`. Search methods take a
 * `Translator` so they can score against the localized text of the active
 * language.
 */
export class HelpCenter {
  private helpContent: Map<string, HelpContent> = new Map();
  private faqs: Map<string, FAQItem> = new Map();
  private tutorials: Map<string, Tutorial> = new Map();
  private userVotes: Record<string, 'helpful' | 'not_helpful'> = {};

  constructor() {
    this.loadDefaultContent();
  }

  /**
   * Initialize and load user preferences.
   */
  async initialize(): Promise<void> {
    await this.loadUserVotes();
  }

  /**
   * Get help content for a screen.
   */
  getHelpForScreen(screenId: string): HelpContent | undefined {
    return this.helpContent.get(screenId);
  }

  /**
   * Search FAQs.
   *
   * `faq.question` / `faq.answer` hold i18n keys, so the `t` translator is used
   * to score against the localized text of the active language. Returned items
   * keep the keys (the UI resolves them with `t`).
   */
  searchFAQs(query: string, t: Translator, category?: FAQCategory): FAQItem[] {
    const normalizedQuery = query.toLowerCase().trim();
    const results: Array<{ item: FAQItem; score: number }> = [];

    for (const faq of this.faqs.values()) {
      // Filter by category if specified
      if (category && faq.category !== category) {
        continue;
      }

      // Resolve localized text for relevance scoring.
      const question = t(faq.question).toLowerCase();
      const answer = t(faq.answer).toLowerCase();

      // Calculate relevance score
      let score = 0;

      if (question.includes(normalizedQuery)) {
        score += 10;
      }
      if (answer.includes(normalizedQuery)) {
        score += 5;
      }
      for (const tag of faq.tags) {
        if (tag.toLowerCase().includes(normalizedQuery)) {
          score += 3;
        }
      }

      if (score > 0 || !normalizedQuery) {
        results.push({ item: faq, score: score || faq.helpful - faq.notHelpful });
      }
    }

    // Sort by score descending
    results.sort((a, b) => b.score - a.score);

    return results.map((r) => r.item);
  }

  /**
   * Get FAQs by category.
   */
  getFAQsByCategory(category: FAQCategory): FAQItem[] {
    return Array.from(this.faqs.values()).filter((faq) => faq.category === category);
  }

  /**
   * Get all FAQ categories with counts.
   *
   * Returns an i18n `labelKey` (`help.categories.<category>`) per category; the
   * UI resolves it with `t(labelKey)`.
   */
  getFAQCategories(): Array<{ category: FAQCategory; count: number; labelKey: string }> {
    const counts: Partial<Record<FAQCategory, number>> = {};

    for (const faq of this.faqs.values()) {
      counts[faq.category] = (counts[faq.category] ?? 0) + 1;
    }

    return Object.entries(counts).map(([category, count]) => ({
      category: category as FAQCategory,
      count: count ?? 0,
      labelKey: `help.categories.${category}`,
    }));
  }

  /**
   * Vote on FAQ helpfulness.
   */
  async voteFAQ(faqId: string, helpful: boolean): Promise<void> {
    const faq = this.faqs.get(faqId);
    if (!faq) return;

    const previousVote = this.userVotes[faqId];

    // Remove previous vote
    if (previousVote === 'helpful') {
      faq.helpful -= 1;
    } else if (previousVote === 'not_helpful') {
      faq.notHelpful -= 1;
    }

    // Add new vote
    if (helpful) {
      faq.helpful += 1;
      this.userVotes[faqId] = 'helpful';
    } else {
      faq.notHelpful += 1;
      this.userVotes[faqId] = 'not_helpful';
    }

    await this.saveUserVotes();
  }

  /**
   * Get user's vote on FAQ.
   */
  getUserVote(faqId: string): 'helpful' | 'not_helpful' | null {
    return this.userVotes[faqId] ?? null;
  }

  /**
   * Get all tutorials.
   */
  getAllTutorials(): Tutorial[] {
    return Array.from(this.tutorials.values());
  }

  /**
   * Get tutorials by category.
   */
  getTutorialsByCategory(category: FAQCategory): Tutorial[] {
    return Array.from(this.tutorials.values()).filter((t) => t.category === category);
  }

  /**
   * Get a tutorial by ID.
   */
  getTutorial(id: string): Tutorial | undefined {
    return this.tutorials.get(id);
  }

  /**
   * Search tutorials.
   *
   * `title` / `description` hold i18n keys, so the `t` translator resolves them
   * for matching against the active language. Returned tutorials keep the keys.
   */
  searchTutorials(query: string, t: Translator): Tutorial[] {
    const normalizedQuery = query.toLowerCase().trim();

    return Array.from(this.tutorials.values()).filter(
      (tut) =>
        t(tut.title).toLowerCase().includes(normalizedQuery) ||
        t(tut.description).toLowerCase().includes(normalizedQuery)
    );
  }

  // Private methods

  private async loadUserVotes(): Promise<void> {
    try {
      const stored = await AsyncStorage.getItem(FAQ_VOTES_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        // Validate structure before assignment
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
          this.userVotes = parsed;
        }
      }
    } catch (error) {
      // Handle corrupted or invalid JSON data gracefully - start fresh
      console.warn('Failed to parse FAQ votes from storage, resetting to default:', error);
      this.userVotes = {};
    }
  }

  private async saveUserVotes(): Promise<void> {
    await AsyncStorage.setItem(FAQ_VOTES_KEY, JSON.stringify(this.userVotes));
  }

  private loadDefaultContent(): void {
    // NOTE: All display strings below are i18n KEYS (resolved by the UI with
    // `t(key)`), not literal English. The copy lives in the locale resource
    // files under the `help.*` namespace (`help.content.*`, `help.faqs.*`,
    // `help.categories.*`, `help.tutorials.*`). Only language-independent,
    // structural data (ids, categories, tags, seed vote counts, durations,
    // relations) is hard-coded here.

    // Help content for screens
    const helpContents: HelpContent[] = [
      {
        id: 'dashboard',
        screen: 'Dashboard',
        title: 'help.content.dashboard.title',
        shortDescription: 'help.content.dashboard.shortDescription',
        fullContent: 'help.content.dashboard.fullContent',
        relatedFAQs: ['faq-1', 'faq-2'],
        // Note: In production, lastUpdated should be fetched dynamically from CMS/API
        // to reflect actual content updates. This static value is temporary seed data.
        lastUpdated: '2025-06-01',
      },
      {
        id: 'buildings',
        screen: 'Buildings',
        title: 'help.content.buildings.title',
        shortDescription: 'help.content.buildings.shortDescription',
        fullContent: 'help.content.buildings.fullContent',
        relatedFAQs: ['faq-3'],
        lastUpdated: '2025-06-01',
      },
      {
        id: 'faults',
        screen: 'Faults',
        title: 'help.content.faults.title',
        shortDescription: 'help.content.faults.shortDescription',
        fullContent: 'help.content.faults.fullContent',
        relatedFAQs: ['faq-4', 'faq-5'],
        relatedTutorials: ['tutorial-faults'],
        lastUpdated: '2025-06-01',
      },
      {
        id: 'voting',
        screen: 'Voting',
        title: 'help.content.voting.title',
        shortDescription: 'help.content.voting.shortDescription',
        fullContent: 'help.content.voting.fullContent',
        relatedFAQs: ['faq-6'],
        relatedTutorials: ['tutorial-voting'],
        lastUpdated: '2025-06-01',
      },
      {
        id: 'documents',
        screen: 'Documents',
        title: 'help.content.documents.title',
        shortDescription: 'help.content.documents.shortDescription',
        fullContent: 'help.content.documents.fullContent',
        relatedFAQs: ['faq-7'],
        lastUpdated: '2025-06-01',
      },
    ];

    for (const content of helpContents) {
      this.helpContent.set(content.screen, content);
    }

    // FAQs
    // `question` / `answer` are i18n keys (`help.faqs.<id>.{question,answer}`).
    // Note: Vote counts (helpful/notHelpful) are:
    // 1. Initial seed values for demonstration
    // 2. User votes stored locally per-device in AsyncStorage (see FAQ_VOTES_KEY)
    // 3. Not synced across devices or aggregated globally
    // In production, vote counts should be fetched from backend API for accurate
    // cross-device aggregation and analytics.
    const faqItems: FAQItem[] = [
      {
        id: 'faq-1',
        question: 'help.faqs.faq-1.question',
        answer: 'help.faqs.faq-1.answer',
        category: 'getting_started',
        tags: ['buildings', 'dashboard', 'view'],
        helpful: 42,
        notHelpful: 3,
      },
      {
        id: 'faq-2',
        question: 'help.faqs.faq-2.question',
        answer: 'help.faqs.faq-2.answer',
        category: 'notifications',
        tags: ['notifications', 'settings', 'alerts'],
        helpful: 35,
        notHelpful: 5,
      },
      {
        id: 'faq-3',
        question: 'help.faqs.faq-3.question',
        answer: 'help.faqs.faq-3.answer',
        category: 'buildings',
        tags: ['buildings', 'switch', 'select'],
        helpful: 28,
        notHelpful: 2,
      },
      {
        id: 'faq-4',
        question: 'help.faqs.faq-4.question',
        answer: 'help.faqs.faq-4.answer',
        category: 'faults',
        tags: ['faults', 'report', 'maintenance'],
        helpful: 56,
        notHelpful: 4,
        relatedQuestions: ['faq-5'],
      },
      {
        id: 'faq-5',
        question: 'help.faqs.faq-5.question',
        answer: 'help.faqs.faq-5.answer',
        category: 'faults',
        tags: ['faults', 'track', 'status'],
        helpful: 38,
        notHelpful: 2,
        relatedQuestions: ['faq-4'],
      },
      {
        id: 'faq-6',
        question: 'help.faqs.faq-6.question',
        answer: 'help.faqs.faq-6.answer',
        category: 'voting',
        tags: ['voting', 'vote', 'proposal'],
        helpful: 45,
        notHelpful: 3,
      },
      {
        id: 'faq-7',
        question: 'help.faqs.faq-7.question',
        answer: 'help.faqs.faq-7.answer',
        category: 'documents',
        tags: ['documents', 'download', 'files'],
        helpful: 32,
        notHelpful: 4,
      },
      {
        id: 'faq-8',
        question: 'help.faqs.faq-8.question',
        answer: 'help.faqs.faq-8.answer',
        category: 'account',
        tags: ['password', 'account', 'security'],
        helpful: 29,
        notHelpful: 1,
      },
      {
        id: 'faq-9',
        question: 'help.faqs.faq-9.question',
        answer: 'help.faqs.faq-9.answer',
        category: 'security',
        tags: ['security', 'privacy', 'data'],
        helpful: 51,
        notHelpful: 2,
      },
      {
        id: 'faq-10',
        question: 'help.faqs.faq-10.question',
        answer: 'help.faqs.faq-10.answer',
        category: 'security',
        tags: ['2fa', 'security', 'authentication'],
        helpful: 37,
        notHelpful: 3,
      },
    ];

    for (const faq of faqItems) {
      this.faqs.set(faq.id, faq);
    }

    // Tutorials
    // `title` / `description` and each step's `title` / `content` are i18n keys
    // under `help.tutorials.<id>.*`.
    const tutorialItems: Tutorial[] = [
      {
        id: 'tutorial-getting-started',
        title: 'help.tutorials.tutorial-getting-started.title',
        description: 'help.tutorials.tutorial-getting-started.description',
        duration: 180,
        category: 'getting_started',
        steps: [
          {
            id: 'step-1',
            title: 'help.tutorials.tutorial-getting-started.steps.step-1.title',
            content: 'help.tutorials.tutorial-getting-started.steps.step-1.content',
          },
          {
            id: 'step-2',
            title: 'help.tutorials.tutorial-getting-started.steps.step-2.title',
            content: 'help.tutorials.tutorial-getting-started.steps.step-2.content',
          },
          {
            id: 'step-3',
            title: 'help.tutorials.tutorial-getting-started.steps.step-3.title',
            content: 'help.tutorials.tutorial-getting-started.steps.step-3.content',
          },
        ],
      },
      {
        id: 'tutorial-faults',
        title: 'help.tutorials.tutorial-faults.title',
        description: 'help.tutorials.tutorial-faults.description',
        duration: 240,
        category: 'faults',
        steps: [
          {
            id: 'step-1',
            title: 'help.tutorials.tutorial-faults.steps.step-1.title',
            content: 'help.tutorials.tutorial-faults.steps.step-1.content',
          },
          {
            id: 'step-2',
            title: 'help.tutorials.tutorial-faults.steps.step-2.title',
            content: 'help.tutorials.tutorial-faults.steps.step-2.content',
          },
          {
            id: 'step-3',
            title: 'help.tutorials.tutorial-faults.steps.step-3.title',
            content: 'help.tutorials.tutorial-faults.steps.step-3.content',
          },
          {
            id: 'step-4',
            title: 'help.tutorials.tutorial-faults.steps.step-4.title',
            content: 'help.tutorials.tutorial-faults.steps.step-4.content',
          },
        ],
      },
      {
        id: 'tutorial-voting',
        title: 'help.tutorials.tutorial-voting.title',
        description: 'help.tutorials.tutorial-voting.description',
        duration: 120,
        category: 'voting',
        steps: [
          {
            id: 'step-1',
            title: 'help.tutorials.tutorial-voting.steps.step-1.title',
            content: 'help.tutorials.tutorial-voting.steps.step-1.content',
          },
          {
            id: 'step-2',
            title: 'help.tutorials.tutorial-voting.steps.step-2.title',
            content: 'help.tutorials.tutorial-voting.steps.step-2.content',
          },
          {
            id: 'step-3',
            title: 'help.tutorials.tutorial-voting.steps.step-3.title',
            content: 'help.tutorials.tutorial-voting.steps.step-3.content',
          },
        ],
      },
    ];

    for (const tutorial of tutorialItems) {
      this.tutorials.set(tutorial.id, tutorial);
    }
  }
}

// Singleton instance
export const helpCenter = new HelpCenter();
