import React from 'react';
import { useTranslation } from 'react-i18next';
import { StyleSheet, Text, View } from 'react-native';
import type { SectionRegistry } from './registry';
import type { ResolvedScreen } from './types';

const warnedTypes = new Set<string>();

function PlaceholderSection() {
  const { t } = useTranslation();
  return (
    <View style={styles.placeholder}>
      <Text style={styles.placeholderTitle}>{t('layout.placeholderTitle')}</Text>
      <Text style={styles.placeholderBody}>{t('layout.placeholderBody')}</Text>
    </View>
  );
}

export interface LayoutSectionsProps {
  layout: ResolvedScreen;
  registry: SectionRegistry;
}

export function LayoutSections({ layout, registry }: LayoutSectionsProps) {
  return (
    <View style={styles.container}>
      {layout.sections.map((section) => {
        const def = registry[section.type];
        if (!def) {
          if (!warnedTypes.has(section.type)) {
            warnedTypes.add(section.type);
            console.warn(`layout: unknown section type ${section.type} — skipped`);
          }
          return null;
        }

        if (section.presentation === 'placeholder') {
          return (
            <View key={section.type} style={styles.sectionWrapper}>
              <PlaceholderSection />
            </View>
          );
        }

        const Component = def.component;
        return (
          <View key={section.type} style={styles.sectionWrapper}>
            <Component mode={section.mode} props={section.props} />
          </View>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  container: { gap: 0 },
  sectionWrapper: {},
  placeholder: {
    margin: 16,
    padding: 16,
    backgroundColor: '#f5f5f5',
    borderRadius: 12,
    alignItems: 'center',
  },
  placeholderTitle: { fontSize: 16, fontWeight: '600', color: '#666', marginBottom: 4 },
  placeholderBody: { fontSize: 14, color: '#999' },
});
