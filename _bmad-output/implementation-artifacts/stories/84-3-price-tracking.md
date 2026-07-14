# Story 84.3: Price Tracking for Favorites

Status: done

## Story

As a **Reality Portal user**,
I want to **be notified when prices change on my favorite listings**,
So that **I can act quickly on price drops or be aware of increases**.

## Acceptance Criteria

1. **AC-1: Price History Tracking**
   - Given a listing price is updated
   - When the update is processed
   - Then the previous price is stored in history
   - And the change percentage is calculated

2. **AC-2: Price Drop Notification**
   - Given I have favorited a listing
   - When the price drops
   - Then I receive a notification
   - And the notification shows old and new price
   - And the percentage decrease is shown

3. **AC-3: Price Increase Notification**
   - Given I have favorited a listing
   - When the price increases
   - Then I receive a notification (if enabled)
   - And the increase amount is shown

4. **AC-4: Price History View**
   - Given I am viewing a listing
   - When I check price history
   - Then I see all historical prices
   - And dates of changes are shown
   - And a price chart is displayed

5. **AC-5: Notification Preferences**
   - Given I want to control notifications
   - When I configure price alerts
   - Then I can enable/disable price drop alerts
   - And I can enable/disable price increase alerts
   - And I can set minimum change threshold

## Tasks / Subtasks

- [ ] Task 1: Create Price History Table (AC: 1)
  - [ ] 1.1 Create `listing_price_history` migration
  - [ ] 1.2 Store price, timestamp, change percentage
  - [ ] 1.3 Link to listing via foreign key
  - [ ] 1.4 Add indexes for efficient queries

- [ ] Task 2: Implement Price Change Detection (AC: 1)
  - [ ] 2.1 Update `/backend/crates/db/src/repositories/portal.rs:324-358`
  - [ ] 2.2 Compare new price with current price on update
  - [ ] 2.3 Calculate change percentage
  - [ ] 2.4 Insert history record
  - [ ] 2.5 Emit price change event

- [ ] Task 3: Implement Notification Service (AC: 2, 3)
  - [ ] 3.1 Create price notification service
  - [ ] 3.2 Find users who favorited listing
  - [ ] 3.3 Filter by notification preferences
  - [ ] 3.4 Send push and/or email notification
  - [ ] 3.5 Throttle notifications per user

- [ ] Task 4: Create Price History API (AC: 4)
  - [ ] 4.1 Create GET `/api/v1/listings/{id}/price-history`
  - [ ] 4.2 Return price changes over time
  - [ ] 4.3 Include change percentages
  - [ ] 4.4 Support date range filtering

- [ ] Task 5: Create Notification Preferences API (AC: 5)
  - [ ] 5.1 Add price alert fields to notification preferences
  - [ ] 5.2 Create update endpoint for preferences
  - [ ] 5.3 Support threshold configuration
  - [ ] 5.4 Default to enabled for price drops

- [ ] Task 6: Frontend Price History Display (AC: 4)
  - [ ] 6.1 Create price history chart component
  - [ ] 6.2 Add to listing detail page
  - [ ] 6.3 Show price change indicators
  - [ ] 6.4 Mobile-friendly display

## Dev Notes

### Architecture Requirements
- Track price changes with timestamps
- Efficient queries for user favorites
- Batch notification sending
- Configurable alert thresholds

### Technical Specifications
- Price history retention: 2 years
- Notification throttle: Max 1 per listing per day
- Default threshold: 0% (all changes)
- Batch size: 100 users per notification batch

### Existing TODO Reference
```rust
// backend/crates/db/src/repositories/portal.rs:324-358
// TODO: Implement price tracking
// - Store price history on listing updates
// - Calculate price change percentage
// - Trigger notifications for favorites
```

### Database Schema
```sql
CREATE TABLE listing_price_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    price DECIMAL(15,2) NOT NULL,
    previous_price DECIMAL(15,2),
    change_percentage DECIMAL(5,2),
    change_type VARCHAR(20), -- 'increase', 'decrease', 'initial'
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source VARCHAR(50) -- 'manual', 'import', 'external'
);

CREATE INDEX idx_price_history_listing ON listing_price_history(listing_id);
CREATE INDEX idx_price_history_date ON listing_price_history(recorded_at);
CREATE INDEX idx_price_history_change ON listing_price_history(change_type);
```

### Price Change Detection
```rust
impl ListingRepository {
    pub async fn update_price(
        &self,
        listing_id: Uuid,
        new_price: Decimal,
        source: &str,
    ) -> Result<Option<PriceChange>, DbError> {
        let current = self.get_current_price(listing_id).await?;

        if let Some(current_price) = current {
            if current_price == new_price {
                return Ok(None); // No change
            }

            let change_percentage = ((new_price - current_price) / current_price * Decimal::from(100))
                .round_dp(2);

            let change_type = if new_price < current_price {
                "decrease"
            } else {
                "increase"
            };

            // Insert history record
            sqlx::query!(
                r#"
                INSERT INTO listing_price_history
                (listing_id, price, previous_price, change_percentage, change_type, source)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                listing_id,
                new_price,
                current_price,
                change_percentage,
                change_type,
                source
            )
            .execute(&self.pool)
            .await?;

            // Update listing price
            sqlx::query!(
                "UPDATE listings SET price = $1, updated_at = NOW() WHERE id = $2",
                new_price,
                listing_id
            )
            .execute(&self.pool)
            .await?;

            return Ok(Some(PriceChange {
                listing_id,
                old_price: current_price,
                new_price,
                change_percentage,
                change_type: change_type.to_string(),
            }));
        }

        // Initial price
        sqlx::query!(
            r#"
            INSERT INTO listing_price_history
            (listing_id, price, change_type, source)
            VALUES ($1, $2, 'initial', $3)
            "#,
            listing_id,
            new_price,
            source
        )
        .execute(&self.pool)
        .await?;

        Ok(None)
    }
}
```

### Notification Service
```rust
pub struct PriceAlertService {
    notification_service: Arc<NotificationService>,
    favorites_repo: Arc<FavoritesRepository>,
    preferences_repo: Arc<PreferencesRepository>,
}

impl PriceAlertService {
    pub async fn process_price_change(&self, change: PriceChange) -> Result<(), ServiceError> {
        // Find users who favorited this listing
        let favorited_by = self.favorites_repo
            .find_users_by_listing(change.listing_id)
            .await?;

        for user_id in favorited_by {
            let prefs = self.preferences_repo
                .get_price_alert_preferences(user_id)
                .await?;

            // Check if notification should be sent
            let should_notify = match change.change_type.as_str() {
                "decrease" => prefs.notify_price_drops,
                "increase" => prefs.notify_price_increases,
                _ => false,
            };

            if should_notify && change.change_percentage.abs() >= prefs.min_change_threshold {
                self.notification_service.send_price_alert(
                    user_id,
                    PriceAlert {
                        listing_id: change.listing_id,
                        listing_title: change.listing_title.clone(),
                        old_price: change.old_price,
                        new_price: change.new_price,
                        change_percentage: change.change_percentage,
                        change_type: change.change_type.clone(),
                    },
                ).await?;
            }
        }

        Ok(())
    }
}
```

### File List (to create/modify)

**Create:**
- `/backend/crates/db/migrations/NNNN_create_price_history.sql`
- `/backend/crates/db/src/repositories/price_history.rs`
- `/backend/servers/reality-server/src/services/price_alert.rs`
- `/frontend/apps/reality-web/src/components/PriceHistoryChart.tsx`

**Modify:**
- `/backend/crates/db/src/repositories/portal.rs` - Price change detection
- `/backend/crates/db/src/repositories/mod.rs` - Export module
- `/backend/servers/reality-server/src/routes/listings.rs` - Add price history endpoint
- `/mobile-native/shared/src/commonMain/kotlin/.../domain/PriceHistory.kt`

### Notification Preferences
```rust
struct PriceAlertPreferences {
    notify_price_drops: bool,      // Default: true
    notify_price_increases: bool,  // Default: false
    min_change_threshold: Decimal, // Default: 0 (%)
    notification_channel: String,  // 'push', 'email', 'both'
}
```

### Dependencies
- Epic 2B (Notifications) - Notification delivery
- Story 82.4 (Listing Detail) - Frontend integration

### References
- [Source: backend/crates/db/src/repositories/portal.rs:324-358]
- [UC-40: Reality Portal Favorites]
