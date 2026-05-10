/**
 * Tour overlay component for interactive onboarding.
 *
 * Epic 50 - Story 50.1: Interactive Onboarding Tour
 */
import { useCallback, useEffect, useState } from 'react';
import { Animated, Dimensions, Modal, Pressable, StyleSheet, Text, View } from 'react-native';

import { tourManager } from '../../onboarding';
import type { OnboardingState, TourConfig, TourStep } from '../../onboarding/types';
import { colors } from '../../screens/shared/screenStyles';

interface TourOverlayProps {
  visible: boolean;
  onClose: () => void;
}

const { width: SCREEN_WIDTH, height: SCREEN_HEIGHT } = Dimensions.get('window');

export function TourOverlay({ visible, onClose }: TourOverlayProps) {
  const [currentStep, setCurrentStep] = useState<TourStep | null>(null);
  const [tourConfig, setTourConfig] = useState<TourConfig | null>(null);
  const [progress, setProgress] = useState<{ current: number; total: number }>({
    current: 0,
    total: 0,
  });
  const [fadeAnim] = useState(new Animated.Value(0));

  useEffect(() => {
    const unsubscribe = tourManager.subscribe((state: OnboardingState) => {
      if (state.currentTour) {
        const tour = tourManager.getTour(state.currentTour);
        const tourProgress = state.tours[state.currentTour];

        setTourConfig(tour ?? null);
        setCurrentStep(tourManager.getCurrentStep());

        if (tour && tourProgress) {
          setProgress({
            current: tourProgress.currentStepIndex + 1,
            total: tour.steps.length,
          });
        }
      } else {
        setCurrentStep(null);
        setTourConfig(null);
      }
    });

    // Initial load
    const state = tourManager.getState();
    if (state.currentTour) {
      const tour = tourManager.getTour(state.currentTour);
      const tourProgress = state.tours[state.currentTour];

      setTourConfig(tour ?? null);
      setCurrentStep(tourManager.getCurrentStep());

      if (tour && tourProgress) {
        setProgress({
          current: tourProgress.currentStepIndex + 1,
          total: tour.steps.length,
        });
      }
    }

    return unsubscribe;
  }, []);

  useEffect(() => {
    if (visible && currentStep) {
      Animated.timing(fadeAnim, {
        toValue: 1,
        duration: 300,
        useNativeDriver: true,
      }).start();
    } else {
      fadeAnim.setValue(0);
    }
  }, [visible, currentStep, fadeAnim]);

  const handleNext = useCallback(async () => {
    const nextStep = await tourManager.nextStep();
    if (!nextStep) {
      onClose();
    }
  }, [onClose]);

  const handlePrevious = useCallback(async () => {
    await tourManager.previousStep();
  }, []);

  const handleSkip = useCallback(async () => {
    await tourManager.skipTour();
    onClose();
  }, [onClose]);

  if (!visible || !currentStep || !tourConfig) {
    return null;
  }

  const getTooltipPosition = (): {
    top?: number;
    bottom?: number;
    left?: number;
    right?: number;
  } => {
    const placement = currentStep.placement ?? 'center';

    switch (placement) {
      case 'top':
        return { bottom: SCREEN_HEIGHT / 2 + 50, left: 20, right: 20 };
      case 'bottom':
        return { top: SCREEN_HEIGHT / 2 + 50, left: 20, right: 20 };
      case 'left':
        return { top: SCREEN_HEIGHT / 3, left: 20, right: SCREEN_WIDTH / 2 + 20 };
      case 'right':
        return { top: SCREEN_HEIGHT / 3, left: SCREEN_WIDTH / 2 + 20, right: 20 };
      default:
        return { top: SCREEN_HEIGHT / 3, left: 20, right: 20 };
    }
  };

  const tooltipPosition = getTooltipPosition();

  return (
    <Modal visible={visible} transparent animationType="none">
      <Animated.View style={[styles.overlay, { opacity: fadeAnim }]}>
        <Pressable style={styles.backdrop} onPress={handleSkip} />

        <Animated.View style={[styles.tooltip, tooltipPosition]}>
          <View style={styles.header}>
            <Text style={styles.stepIndicator}>
              {progress.current} / {progress.total}
            </Text>
            <Pressable onPress={handleSkip} style={styles.skipButton}>
              <Text style={styles.skipText}>Skip Tour</Text>
            </Pressable>
          </View>

          <Text style={styles.title}>{currentStep.title}</Text>
          <Text style={styles.content}>{currentStep.content}</Text>

          {currentStep.actionHint && (
            <View style={styles.actionHint}>
              <Text style={styles.actionHintText}>💡 {currentStep.actionHint}</Text>
            </View>
          )}

          <View style={styles.progressBar}>
            <View
              style={[
                styles.progressFill,
                { width: `${(progress.current / progress.total) * 100}%` },
              ]}
            />
          </View>

          <View style={styles.navigation}>
            {progress.current > 1 ? (
              <Pressable style={styles.navButton} onPress={handlePrevious}>
                <Text style={styles.navButtonText}>← Previous</Text>
              </Pressable>
            ) : (
              <View style={styles.navButton} />
            )}

            <Pressable style={styles.nextButton} onPress={handleNext}>
              <Text style={styles.nextButtonText}>
                {progress.current === progress.total ? 'Done' : 'Next →'}
              </Text>
            </Pressable>
          </View>
        </Animated.View>

        {currentStep.highlightArea && (
          <View
            style={[
              styles.highlight,
              {
                top: currentStep.highlightArea.y,
                left: currentStep.highlightArea.x,
                width: currentStep.highlightArea.width,
                height: currentStep.highlightArea.height,
              },
            ]}
          />
        )}
      </Animated.View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  overlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.7)',
  },
  backdrop: {
    ...StyleSheet.absoluteFill,
  },
  tooltip: {
    position: 'absolute',
    backgroundColor: colors.white,
    borderRadius: 16,
    padding: 20,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 10,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  stepIndicator: {
    fontSize: 14,
    color: colors.textMuted,
    fontWeight: '500',
  },
  skipButton: {
    padding: 4,
  },
  skipText: {
    fontSize: 14,
    color: colors.textSubtle,
  },
  title: {
    fontSize: 20,
    fontWeight: 'bold',
    color: colors.text,
    marginBottom: 8,
  },
  content: {
    fontSize: 16,
    color: colors.textMuted,
    lineHeight: 24,
    marginBottom: 16,
  },
  actionHint: {
    backgroundColor: colors.warningBg,
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
  },
  actionHintText: {
    fontSize: 14,
    color: colors.statusInProgressInk,
  },
  progressBar: {
    height: 4,
    backgroundColor: colors.border,
    borderRadius: 2,
    marginBottom: 16,
    overflow: 'hidden',
  },
  progressFill: {
    height: '100%',
    backgroundColor: colors.accent,
    borderRadius: 2,
  },
  navigation: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  navButton: {
    paddingVertical: 10,
    paddingHorizontal: 16,
    minWidth: 100,
  },
  navButtonText: {
    fontSize: 16,
    color: colors.textMuted,
  },
  nextButton: {
    backgroundColor: colors.accent,
    paddingVertical: 12,
    paddingHorizontal: 24,
    borderRadius: 8,
  },
  nextButtonText: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.white,
  },
  highlight: {
    position: 'absolute',
    borderWidth: 3,
    borderColor: colors.warning,
    borderRadius: 8,
    backgroundColor: 'rgba(251, 191, 36, 0.1)',
  },
});
