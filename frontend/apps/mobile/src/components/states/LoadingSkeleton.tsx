/**
 * LoadingSkeleton — animated shimmer placeholder for mobile lists/blocks.
 */

import { useEffect, useRef } from 'react';
import { Animated, StyleSheet, View, type ViewStyle } from 'react-native';

export interface LoadingSkeletonProps {
  /** Render N stacked card placeholders. */
  cards?: number;
  /** Render N text-line placeholders. */
  lines?: number;
  /** Explicit width (defaults to '100%'). */
  width?: ViewStyle['width'];
  /** Explicit height (defaults to 16). */
  height?: ViewStyle['height'];
}

const useShimmer = () => {
  const value = useRef(new Animated.Value(0)).current;
  useEffect(() => {
    const loop = Animated.loop(
      Animated.timing(value, {
        toValue: 1,
        duration: 1500,
        useNativeDriver: false,
      })
    );
    loop.start();
    return () => loop.stop();
  }, [value]);
  return value;
};

function SkeletonBlock({
  width,
  height,
  radius = 6,
}: { width: ViewStyle['width']; height: ViewStyle['height']; radius?: number }) {
  const shimmer = useShimmer();
  const backgroundColor = shimmer.interpolate({
    inputRange: [0, 0.5, 1],
    outputRange: ['#f3f4f6', '#e5e7eb', '#f3f4f6'],
  });
  return (
    <Animated.View
      style={{ width, height, backgroundColor, borderRadius: radius }}
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
    />
  );
}

export function LoadingSkeleton({
  cards,
  lines,
  width = '100%',
  height = 16,
}: LoadingSkeletonProps) {
  if (cards != null) {
    return (
      <View accessibilityRole="progressbar" accessibilityLabel="Loading">
        {Array.from({ length: cards }).map((_, i) => (
          <View key={`card-${i}`} style={styles.cardWrap}>
            <SkeletonBlock width="100%" height={100} radius={12} />
          </View>
        ))}
      </View>
    );
  }
  if (lines != null) {
    return (
      <View accessibilityRole="progressbar" accessibilityLabel="Loading">
        {Array.from({ length: lines }).map((_, i) => (
          <View key={`line-${i}`} style={styles.lineWrap}>
            <SkeletonBlock width={i === lines - 1 ? '70%' : '100%'} height={height} />
          </View>
        ))}
      </View>
    );
  }
  return (
    <View accessibilityRole="progressbar" accessibilityLabel="Loading">
      <SkeletonBlock width={width} height={height} />
    </View>
  );
}

const styles = StyleSheet.create({
  cardWrap: { marginBottom: 12 },
  lineWrap: { marginBottom: 8 },
});
