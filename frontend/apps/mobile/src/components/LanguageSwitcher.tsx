import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { type Locale, localeFlags, localeNames, locales } from '../i18n';
import { colors } from '../screens/shared/screenStyles';

export function LanguageSwitcher() {
  const { i18n, t } = useTranslation();
  const [modalVisible, setModalVisible] = useState(false);
  const currentLocale = i18n.language as Locale;

  const handleLanguageChange = (locale: Locale) => {
    i18n.changeLanguage(locale);
    setModalVisible(false);
  };

  return (
    <>
      <Pressable style={styles.button} onPress={() => setModalVisible(true)}>
        <Text style={styles.buttonText}>
          {localeFlags[currentLocale]} {localeNames[currentLocale]}
        </Text>
      </Pressable>

      <Modal
        animationType="slide"
        transparent={true}
        visible={modalVisible}
        onRequestClose={() => setModalVisible(false)}
      >
        <View style={styles.modalOverlay}>
          <View style={styles.modalContent}>
            <Text style={styles.modalTitle}>{t('settings.language')}</Text>
            {locales.map((locale) => (
              <Pressable
                key={locale}
                style={[
                  styles.languageOption,
                  currentLocale === locale && styles.languageOptionActive,
                ]}
                onPress={() => handleLanguageChange(locale)}
              >
                <Text style={styles.languageFlag}>{localeFlags[locale]}</Text>
                <Text
                  style={[
                    styles.languageName,
                    currentLocale === locale && styles.languageNameActive,
                  ]}
                >
                  {localeNames[locale]}
                </Text>
                {currentLocale === locale && <Text style={styles.checkmark}>✓</Text>}
              </Pressable>
            ))}
            <Pressable style={styles.closeButton} onPress={() => setModalVisible(false)}>
              <Text style={styles.closeButtonText}>{t('common.cancel')}</Text>
            </Pressable>
          </View>
        </View>
      </Modal>
    </>
  );
}

const styles = StyleSheet.create({
  button: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 12,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
  },
  buttonText: {
    fontSize: 16,
    color: colors.textSecondary,
  },
  modalOverlay: {
    flex: 1,
    justifyContent: 'flex-end',
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
  },
  modalContent: {
    backgroundColor: colors.white,
    borderTopLeftRadius: 20,
    borderTopRightRadius: 20,
    padding: 20,
    paddingBottom: 40,
  },
  modalTitle: {
    fontSize: 20,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 16,
    textAlign: 'center',
  },
  languageOption: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 16,
    borderRadius: 12,
    marginBottom: 8,
  },
  languageOptionActive: {
    backgroundColor: colors.accentSoft,
  },
  languageFlag: {
    fontSize: 24,
    marginRight: 12,
  },
  languageName: {
    fontSize: 16,
    color: colors.textSecondary,
    flex: 1,
  },
  languageNameActive: {
    color: colors.accent,
    fontWeight: '600',
  },
  checkmark: {
    fontSize: 18,
    color: colors.accent,
  },
  closeButton: {
    marginTop: 8,
    padding: 16,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 12,
    alignItems: 'center',
  },
  closeButtonText: {
    fontSize: 16,
    color: colors.textMuted,
    fontWeight: '500',
  },
});
