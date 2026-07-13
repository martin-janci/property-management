package three.two.bit.ppt.reality.favorites

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * Immutable UI state for the Price-Tracking surface.
 *
 * The screen renders two things off this single value: the user's favorited
 * listings annotated with their price movement (so a card can show a "was
 * €X → now €Y" badge), and the reverse-chronological feed of price-change
 * alerts the server has queued for the user. Everything the Compose (Android)
 * or SwiftUI (iOS) surface needs is derived here so the screen stays a pure
 * `collectAsState()` render + intent wiring — mirroring [ListingDetailUiState].
 */
data class PriceTrackingUiState(
    val isLoading: Boolean = true,
    val errorMessage: String? = null,
    /** `false` once we learn there is no session — favorites/alerts require auth. */
    val isAuthenticated: Boolean = true,
    /** All favorites (each carries the embedded flat listing summary + price fields). */
    val favorites: List<FavoriteEntry> = emptyList(),
    /** Newest-first feed of favorite price/status alerts. */
    val alerts: List<FavoriteAlert> = emptyList(),
    /** Total pending (unread) alerts, independent of the loaded page window. */
    val unreadCount: Long = 0,
    /** True while a mark-read / mark-all-read write is in flight. */
    val isMutating: Boolean = false,
) {
    /** Favorites whose price moved since they were saved — the price-tracking highlights. */
    val priceChangedFavorites: List<FavoriteEntry>
        get() = favorites.filter { it.priceChanged }

    /** How many favorites have price-drop alerts armed. */
    val priceAlertEnabledCount: Int
        get() = favorites.count { it.priceAlertEnabled }

    /** The alert feed narrowed to price movements (drops/rises), excluding status-only alerts. */
    val priceAlerts: List<FavoriteAlert>
        get() = alerts.filter { it.isPriceChange }

    /** True when there is nothing to track and no alerts — drives the empty state. */
    val isEmpty: Boolean
        get() = favorites.isEmpty() && alerts.isEmpty()
}

/**
 * State-holder / view-model for the Reality-portal Price-Tracking screen — the mobile surface for
 * price tracking on favorited listings (gap-84-3).
 *
 * It loads the user's favorites (each embeds `price_changed` / `price_change_percentage` /
 * `price_alert_enabled`) together with the queued favorite-alert feed
 * (`GET /api/v1/favorites/alerts`), and owns the optimistic mark-read / mark-all-read writes
 * (`POST /api/v1/favorites/alerts/{id}/read`, `.../read-all`).
 *
 * Like [ListingDetailViewModel] it is a plain, framework-agnostic state-holder (no
 * `androidx.lifecycle` / Android types) so it lives in `commonMain`, is unit-testable off a
 * `TestScope`, and is reused as-is by the iOS target. DI is by constructor injection; [create]
 * keeps repository construction out of the UI layer. The favorites repository carries the bearer
 * token, so it is rebuilt (not the whole view-model) on a session change — see [updateAuth].
 *
 * @param favoritesRepositoryFactory builds a [FavoritesRepository] carrying the given session
 *   token. Held as a factory so [updateAuth] can rebuild the auth-scoped repository on
 *   login/logout without re-creating the view-model.
 * @param scope the coroutine scope loads/writes launch into (a `rememberCoroutineScope()` on
 *   Android; the `TestScope` from `runTest` in tests).
 * @param initialSessionToken the session token at construction time (`null` == unauthenticated).
 * @param errorMapper maps a caught [Throwable] to a user-facing message — injected because the
 *   Android strings come from `stringResource(...)`, unreachable from `commonMain`.
 * @param alertsPageLimit page size requested from `GET /api/v1/favorites/alerts` (server clamps to
 *   `[1, 200]`).
 */
class PriceTrackingViewModel(
    private val favoritesRepositoryFactory: (sessionToken: String?) -> FavoritesRepository,
    private val scope: CoroutineScope,
    initialSessionToken: String?,
    private val errorMapper: (Throwable) -> String,
    private val alertsPageLimit: Int = 100,
) {
    private val _state =
        MutableStateFlow(PriceTrackingUiState(isAuthenticated = initialSessionToken != null))
    val state: StateFlow<PriceTrackingUiState> = _state.asStateFlow()

    private var currentSessionToken: String? = initialSessionToken
    private var favoritesRepository: FavoritesRepository =
        favoritesRepositoryFactory(initialSessionToken)

    private var started = false

    /**
     * Kick off the initial load. Idempotent per instance so it can be safely driven from a
     * `LaunchedEffect(viewModel)`.
     */
    fun start() {
        if (started) return
        started = true
        load()
    }

    /** Pull-to-refresh: reload favorites + alerts. */
    fun refresh() = load()

    /** Retry after a failed load. */
    fun retry() = load()

    private fun load() {
        if (currentSessionToken == null) {
            // Favorites/alerts require auth; render the signed-out empty state rather than firing
            // requests that will only 401.
            _state.update {
                it.copy(
                    isLoading = false,
                    isAuthenticated = false,
                    favorites = emptyList(),
                    alerts = emptyList(),
                    unreadCount = 0,
                    errorMessage = null,
                )
            }
            return
        }
        _state.update { it.copy(isLoading = true, errorMessage = null, isAuthenticated = true) }
        val tokenAtLaunch = currentSessionToken
        val repositoryAtLaunch = favoritesRepository
        scope.launch {
            val favoritesResult = repositoryAtLaunch.getFavorites()
            val alertsResult = repositoryAtLaunch.getFavoriteAlerts(limit = alertsPageLimit)

            // A session change mid-flight supersedes this load; don't clobber the new session.
            if (currentSessionToken != tokenAtLaunch) return@launch

            favoritesResult.fold(
                onSuccess = { favorites ->
                    // Favorites are the primary payload. Alerts are best-effort: a failure there
                    // shouldn't blank the whole screen, so we keep whatever we last had.
                    val alerts = alertsResult.getOrNull()
                    _state.update {
                        it.copy(
                            isLoading = false,
                            errorMessage = null,
                            favorites = favorites.favorites,
                            alerts = alerts?.alerts ?: it.alerts,
                            unreadCount = alerts?.unreadCount ?: it.unreadCount,
                        )
                    }
                },
                onFailure = { e ->
                    _state.update { it.copy(isLoading = false, errorMessage = errorMapper(e)) }
                },
            )
        }
    }

    /**
     * React to a session change (login / logout): rebuild the auth-scoped repository with the new
     * token and reload. A no-op when the token is unchanged, so it is safe to drive from a
     * `LaunchedEffect(sessionToken)` (which also fires on first composition).
     */
    fun updateAuth(sessionToken: String?) {
        if (sessionToken == currentSessionToken) return
        currentSessionToken = sessionToken
        favoritesRepository = favoritesRepositoryFactory(sessionToken)
        load()
    }

    /**
     * Optimistically mark a single alert read: flip it to `sent` and decrement the unread count
     * immediately, then reconcile with the server. On failure the change rolls back so the badge
     * never lies. No-op when unauthenticated, when the alert is unknown, or when it is already read.
     */
    fun onMarkAlertRead(alertId: String) {
        if (currentSessionToken == null) return
        val current = _state.value
        val target = current.alerts.firstOrNull { it.id == alertId } ?: return
        if (!target.isUnread) return

        val previousAlerts = current.alerts
        val previousUnread = current.unreadCount
        val tokenAtLaunch = currentSessionToken
        val repositoryAtLaunch = favoritesRepository

        _state.update {
            it.copy(
                alerts = it.alerts.map { a -> if (a.id == alertId) a.copy(status = "sent") else a },
                unreadCount = (it.unreadCount - 1).coerceAtLeast(0L),
                isMutating = true,
            )
        }
        scope.launch {
            val result = repositoryAtLaunch.markFavoriteAlertRead(alertId)
            if (currentSessionToken != tokenAtLaunch) return@launch
            _state.update {
                if (result.isFailure) {
                    it.copy(alerts = previousAlerts, unreadCount = previousUnread, isMutating = false)
                } else {
                    it.copy(isMutating = false)
                }
            }
        }
    }

    /**
     * Optimistically mark every pending alert read. On failure it reloads from the server rather
     * than trying to reconstruct the prior per-alert state.
     */
    fun onMarkAllRead() {
        if (currentSessionToken == null) return
        val current = _state.value
        if (current.unreadCount == 0L && current.alerts.none { it.isUnread }) return

        val tokenAtLaunch = currentSessionToken
        val repositoryAtLaunch = favoritesRepository

        _state.update {
            it.copy(
                alerts = it.alerts.map { a -> if (a.isUnread) a.copy(status = "sent") else a },
                unreadCount = 0,
                isMutating = true,
            )
        }
        scope.launch {
            val result = repositoryAtLaunch.markAllFavoriteAlertsRead()
            if (currentSessionToken != tokenAtLaunch) return@launch
            if (result.isFailure) {
                _state.update { it.copy(isMutating = false) }
                load()
            } else {
                _state.update { it.copy(isMutating = false) }
            }
        }
    }

    companion object {
        /**
         * Builds the view-model together with its [FavoritesRepository], keeping repository
         * construction out of the UI layer. `sessionToken == null` is treated as unauthenticated.
         */
        fun create(
            baseUrl: String,
            sessionToken: String?,
            scope: CoroutineScope,
            errorMapper: (Throwable) -> String,
        ): PriceTrackingViewModel =
            PriceTrackingViewModel(
                favoritesRepositoryFactory = { token ->
                    FavoritesRepository(baseUrl = baseUrl, sessionToken = token)
                },
                scope = scope,
                initialSessionToken = sessionToken,
                errorMapper = errorMapper,
            )
    }
}
