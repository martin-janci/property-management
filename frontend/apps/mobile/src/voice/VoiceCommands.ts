/**
 * Voice command parsing and intent recognition.
 *
 * Epic 49 - Story 49.2: Voice Assistant Integration
 */
import i18n from 'i18next';
import type {
  ParsedVoiceCommand,
  VoiceActionResult,
  VoiceCommandParams,
  VoiceIntent,
} from './types';

/**
 * Spoken confirmation / error copy is localized through the shared i18next
 * singleton (the same instance configured in `../i18n`), mirroring the
 * module-level convention in `../i18n/format.ts`. These helpers are plain
 * functions rather than React components, so they read the app's *active*
 * language from the singleton instead of the `useTranslation` hook — the
 * rendered/spoken string still matches the language the user picked in-app.
 */
function t(key: string, params?: Record<string, string>): string {
  return i18n.t(`voice.${key}`, params ?? {});
}

/**
 * Command patterns for intent recognition.
 */
const COMMAND_PATTERNS: Array<{
  intent: VoiceIntent;
  patterns: RegExp[];
  paramExtractors?: Array<{
    key: keyof VoiceCommandParams;
    pattern: RegExp;
    transform?: (match: string) => string;
  }>;
}> = [
  {
    intent: 'report_fault',
    patterns: [
      /report\s+(a\s+)?(fault|issue|problem)/i,
      /there\s+is\s+(a\s+)?(problem|issue|fault)/i,
      /something\s+(is\s+)?(broken|not\s+working)/i,
      /fault\s+report/i,
      /log\s+(a\s+)?(fault|issue)/i,
    ],
    paramExtractors: [
      {
        key: 'faultCategory',
        pattern:
          /(elevator|lift|plumbing|water|electrical|heating|cooling|security|door|gate|parking|light|noise)/i,
      },
      {
        key: 'location',
        // Match location after "in" or "at", capturing multi-word locations
        pattern:
          /(?:in|at)\s+(?:the\s+)?([a-z0-9][\w\s-]*?)(?=\s+(?:is|has|was|are|were|there|please|can|could)|[.,]|$)/i,
        transform: (match: string) => match.trim(),
      },
      {
        key: 'priority',
        pattern: /(urgent|high\s+priority|emergency|important|low\s+priority)/i,
        transform: (match: string) => {
          if (/urgent|emergency/i.test(match)) return 'urgent';
          if (/high/i.test(match)) return 'high';
          if (/low/i.test(match)) return 'low';
          return 'medium';
        },
      },
    ],
  },
  {
    intent: 'view_announcements',
    patterns: [
      /show\s+(me\s+)?(the\s+)?announcements?/i,
      /what('s|\s+are)\s+(the\s+)?news/i,
      /any\s+announcements?/i,
      /read\s+announcements?/i,
      /open\s+announcements?/i,
    ],
  },
  {
    intent: 'cast_vote',
    patterns: [
      /vote\s+(on|for)/i,
      /cast\s+(a\s+|my\s+)?vote/i,
      /show\s+(me\s+)?(the\s+)?vote/i,
      /open\s+voting/i,
      /any\s+(active\s+)?votes?/i,
    ],
    paramExtractors: [
      {
        key: 'voteId',
        pattern: /vote\s+(?:on|for)\s+(.+?)(?:\s|$)/i,
      },
    ],
  },
  {
    intent: 'view_documents',
    patterns: [
      /show\s+(me\s+)?(the\s+)?documents?/i,
      /open\s+documents?/i,
      /find\s+(a\s+)?document/i,
      /search\s+documents?/i,
    ],
    paramExtractors: [
      {
        key: 'documentType',
        pattern: /(contract|invoice|report|minutes|regulation|certificate)/i,
      },
      {
        key: 'query',
        pattern: /(?:find|search)\s+(?:for\s+)?(.+)/i,
      },
    ],
  },
  {
    intent: 'check_notifications',
    patterns: [
      /show\s+(me\s+)?(my\s+)?notifications?/i,
      /any\s+notifications?/i,
      /check\s+notifications?/i,
      /what('s|\s+are)\s+new/i,
      /any\s+updates?/i,
    ],
  },
  {
    intent: 'view_dashboard',
    patterns: [
      /go\s+(to\s+)?(the\s+)?home/i,
      /open\s+(the\s+)?dashboard/i,
      /show\s+(me\s+)?(the\s+)?dashboard/i,
      /home\s+screen/i,
    ],
  },
  {
    intent: 'open_settings',
    patterns: [/open\s+settings/i, /go\s+to\s+settings/i, /show\s+settings/i, /preferences/i],
  },
];

/**
 * Parse a voice command transcript into a structured command.
 */
export function parseVoiceCommand(transcript: string): ParsedVoiceCommand {
  const normalizedTranscript = transcript.toLowerCase().trim();
  let bestMatch: { intent: VoiceIntent; confidence: number; params: VoiceCommandParams } = {
    intent: 'unknown',
    confidence: 0,
    params: {},
  };

  for (const commandDef of COMMAND_PATTERNS) {
    for (const pattern of commandDef.patterns) {
      const match = normalizedTranscript.match(pattern);
      if (match) {
        // Calculate confidence based on multiple factors:
        // 1. Match coverage (how much of the transcript matches)
        // 2. Match position (earlier matches are more intentional)
        // 3. Pattern specificity (longer patterns are more specific)
        const matchLength = match[0].length;
        const transcriptLength = normalizedTranscript.length;
        const matchStart = match.index ?? 0;

        // Coverage: ratio of matched text to total text
        const coverageScore = matchLength / transcriptLength;
        // Position: prefer matches at the start (1.0 at start, decreasing)
        const positionScore = 1 - matchStart / transcriptLength;
        // Base confidence from pattern match
        const baseConfidence = 0.4;

        // Weighted combination
        const confidence = Math.min(
          1,
          baseConfidence + coverageScore * 0.35 + positionScore * 0.25
        );

        if (confidence > bestMatch.confidence) {
          const params: VoiceCommandParams = {};

          // Extract parameters if extractors are defined
          if (commandDef.paramExtractors) {
            for (const extractor of commandDef.paramExtractors) {
              const paramMatch = normalizedTranscript.match(extractor.pattern);
              if (paramMatch?.[1]) {
                const value = extractor.transform
                  ? extractor.transform(paramMatch[1])
                  : paramMatch[1].trim();
                (params as Record<string, string>)[extractor.key] = value;
              }
            }
          }

          bestMatch = {
            intent: commandDef.intent,
            confidence,
            params,
          };
        }
      }
    }
  }

  return {
    intent: bestMatch.intent,
    confidence: bestMatch.confidence,
    params: bestMatch.params,
    transcript,
  };
}

/**
 * Execute a parsed voice command and return the result.
 */
export function executeVoiceCommand(command: ParsedVoiceCommand): VoiceActionResult {
  switch (command.intent) {
    case 'report_fault':
      return {
        success: true,
        action: 'Navigate to fault reporting',
        navigation: 'ReportFault',
        prefilledData: {
          category: command.params.faultCategory ?? '',
          location: command.params.location ?? '',
          priority: command.params.priority ?? 'medium',
        },
        confirmationMessage: command.params.faultCategory
          ? t('confirmations.reportFaultCategory', { category: command.params.faultCategory })
          : t('confirmations.reportFaultForm'),
      };

    case 'view_announcements':
      return {
        success: true,
        action: 'Navigate to announcements',
        navigation: 'Announcements',
        confirmationMessage: t('confirmations.announcements'),
      };

    case 'cast_vote':
      return {
        success: true,
        action: 'Navigate to voting',
        navigation: 'Voting',
        prefilledData: command.params.voteId ? { voteId: command.params.voteId } : undefined,
        confirmationMessage: t('confirmations.voting'),
      };

    case 'view_documents':
      return {
        success: true,
        action: 'Navigate to documents',
        navigation: 'Documents',
        prefilledData: {
          type: command.params.documentType ?? '',
          query: command.params.query ?? '',
        },
        confirmationMessage: command.params.query
          ? t('confirmations.documentsSearch', { query: command.params.query })
          : t('confirmations.documents'),
      };

    case 'check_notifications':
      return {
        success: true,
        action: 'Check notifications',
        navigation: 'Dashboard',
        confirmationMessage: t('confirmations.notifications'),
      };

    case 'view_dashboard':
      return {
        success: true,
        action: 'Navigate to dashboard',
        navigation: 'Dashboard',
        confirmationMessage: t('confirmations.dashboard'),
      };

    case 'open_settings':
      return {
        success: true,
        action: 'Navigate to settings',
        navigation: 'Settings',
        confirmationMessage: t('confirmations.settings'),
      };

    default:
      return {
        success: false,
        action: 'Unknown command',
        errorMessage: t('errors.unknownCommand'),
      };
  }
}

/**
 * Get suggested voice commands for user guidance.
 */
export function getSuggestedCommands(): Array<{
  phrase: string;
  description: string;
}> {
  return [
    { phrase: 'Report a fault', description: 'Open fault reporting form' },
    { phrase: 'Report elevator problem', description: 'Pre-fill elevator fault' },
    { phrase: 'Show announcements', description: 'View building announcements' },
    { phrase: 'Any active votes?', description: 'Check pending votes' },
    { phrase: 'Open documents', description: 'Browse documents' },
    { phrase: 'Find contract', description: 'Search for contracts' },
    { phrase: 'Check notifications', description: 'View notifications' },
    { phrase: 'Go home', description: 'Return to dashboard' },
  ];
}

/**
 * Format a voice command for display.
 */
export function formatCommandForDisplay(command: ParsedVoiceCommand): string {
  const intentLabels: Record<VoiceIntent, string> = {
    report_fault: 'Report Fault',
    view_announcements: 'Announcements',
    cast_vote: 'Voting',
    view_documents: 'Documents',
    check_notifications: 'Notifications',
    view_dashboard: 'Dashboard',
    open_settings: 'Settings',
    unknown: 'Unknown',
  };

  return `${intentLabels[command.intent]} (${Math.round(command.confidence * 100)}% confident)`;
}
