/**
 * FaultsScreen - main screen listing all faults.
 * Epic 4: Fault Reporting & Resolution (UC-03)
 */

import {
  ActivityIndicator,
  FlatList,
  RefreshControl,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { colors } from '../../../screens/shared/screenStyles';
import { FaultCard, type FaultSummary } from '../components/FaultCard';

interface FaultsScreenProps {
  faults: FaultSummary[];
  isLoading?: boolean;
  isRefreshing?: boolean;
  onRefresh: () => void;
  onFaultPress: (id: string) => void;
  onCreatePress: () => void;
}

export function FaultsScreen({
  faults,
  isLoading,
  isRefreshing,
  onRefresh,
  onFaultPress,
  onCreatePress,
}: FaultsScreenProps) {
  if (isLoading && !isRefreshing) {
    return (
      <View style={styles.loadingContainer}>
        <ActivityIndicator size="large" color={colors.accent} />
      </View>
    );
  }

  return (
    <View style={styles.container}>
      <FlatList
        data={faults}
        keyExtractor={(item) => item.id}
        renderItem={({ item }) => <FaultCard fault={item} onPress={() => onFaultPress(item.id)} />}
        refreshControl={<RefreshControl refreshing={isRefreshing || false} onRefresh={onRefresh} />}
        ListEmptyComponent={
          <View style={styles.emptyContainer}>
            <Text style={styles.emptyText}>No faults reported yet.</Text>
          </View>
        }
        contentContainerStyle={faults.length === 0 ? styles.emptyList : styles.list}
      />

      <TouchableOpacity style={styles.fab} onPress={onCreatePress} activeOpacity={0.8}>
        <Text style={styles.fabText}>+</Text>
      </TouchableOpacity>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.surfaceMuted,
  },
  loadingContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    backgroundColor: colors.surfaceMuted,
  },
  list: {
    paddingTop: 8,
    paddingBottom: 80,
  },
  emptyList: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  emptyContainer: {
    padding: 32,
    alignItems: 'center',
  },
  emptyText: {
    fontSize: 16,
    color: colors.textMuted,
  },
  fab: {
    position: 'absolute',
    right: 20,
    bottom: 20,
    width: 56,
    height: 56,
    borderRadius: 28,
    backgroundColor: colors.accent,
    justifyContent: 'center',
    alignItems: 'center',
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 4,
    elevation: 8,
  },
  fabText: {
    fontSize: 28,
    color: colors.white,
    fontWeight: '300',
  },
});
