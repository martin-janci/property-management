package three.two.bit.ppt.reality.listing

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.inquiry.CreateInquiryRequest
import three.two.bit.ppt.reality.inquiry.InquiryRepository

/**
 * Immutable UI state for the Listing Detail screen.
 *
 * Every field the Compose (or, later, SwiftUI) surface needs is derived from this single value so
 * the screen can be a pure `collectAsState()` render + intent wiring. See [ListingDetailViewModel].
 */
data class ListingDetailUiState(
    val listing: ListingDetail? = null,
    val isLoading: Boolean = true,
    val errorMessage: String? = null,
    val isFavorite: Boolean = false,
    val isFavoriteLoading: Boolean = false,
    val showInquiryDialog: Boolean = false,
    val isInquirySubmitting: Boolean = false,
    val inquiryError: String? = null,
    val showShareSheet: Boolean = false,
)

/** One-shot side effects the UI must react to (navigation, etc.). */
sealed interface ListingDetailEvent {
    /** The inquiry was accepted by the server; the UI should navigate to the inquiries list. */
    data object InquirySubmitted : ListingDetailEvent
}

/**
 * State-holder / view-model for the Reality-portal Listing Detail screen.
 *
 * This hoists the state and side-effects that used to live inline in the `ListingDetailScreen`
 * composable — the churn locus flagged in issue #2079 (PR #2059 follow-up). It owns the two data
 * loads, the optimistic favorite toggle (+ rollback), inquiry submission, and share/dialog
 * visibility, exposing them as an immutable [state] `StateFlow` plus intent methods
 * ([onFavoriteToggle], [onSubmitInquiry], [retry], …). The composable shrinks to
 * `collectAsState()` + intent wiring.
 *
 * It is a plain, framework-agnostic state-holder (no `androidx.lifecycle` / Android types) so it
 * lives in `commonMain`, is unit-testable off a `TestScope`, and can be reused by the iOS target
 * later — mirroring the existing [SearchState] convention. The app currently has no Koin container,
 * so DI is done by constructor injection; [create] keeps repository construction out of the UI
 * layer. Wiring a Koin single-module is left as a separate infra follow-up (tracked in #2079).
 *
 * @param scope the coroutine scope the loads/writes launch into. On Android this is a
 *   `rememberCoroutineScope()`, so in-flight work is cancelled when the screen leaves composition;
 *   in tests it is the `TestScope` from `runTest`.
 * @param favoritesRepositoryFactory builds a [FavoritesRepository] carrying the given session
 *   token. Held as a factory (rather than a ready-made repo) so [updateAuth] can rebuild the
 *   auth-scoped repositories on a login/logout without re-creating the whole view-model — see
 *   issue #2108.
 * @param inquiryRepositoryFactory builds an [InquiryRepository] carrying the given session token.
 * @param initialSessionToken the session token at construction time (`null` == unauthenticated).
 * @param errorMapper maps a caught [Throwable] to a user-facing message — injected because the
 *   Android strings come from `stringResource(...)`, which is not reachable from `commonMain`.
 */
class ListingDetailViewModel(
    private val listingId: String,
    private val listingRepository: ListingRepository,
    private val favoritesRepositoryFactory: (sessionToken: String?) -> FavoritesRepository,
    private val inquiryRepositoryFactory: (sessionToken: String?) -> InquiryRepository,
    private val scope: CoroutineScope,
    initialSessionToken: String?,
    private val errorMapper: (Throwable) -> String,
) {
    private val _state = MutableStateFlow(ListingDetailUiState())
    val state: StateFlow<ListingDetailUiState> = _state.asStateFlow()

    private val _events = MutableSharedFlow<ListingDetailEvent>(extraBufferCapacity = 1)
    val events: SharedFlow<ListingDetailEvent> = _events.asSharedFlow()

    // Auth-scoped state. The favorites/inquiry repositories carry the bearer token, so they are
    // rebuilt (not the whole view-model) whenever the session changes — see [updateAuth].
    private var currentSessionToken: String? = initialSessionToken
    private var isAuthenticated: Boolean = initialSessionToken != null
    private var favoritesRepository: FavoritesRepository =
        favoritesRepositoryFactory(initialSessionToken)
    private var inquiryRepository: InquiryRepository = inquiryRepositoryFactory(initialSessionToken)

    private var started = false

    /**
     * Kick off the initial listing load (and the favorite-state probe when authenticated).
     * Idempotent per instance so it can be safely driven from a `LaunchedEffect(viewModel)`.
     */
    fun start() {
        if (started) return
        started = true
        loadListing()
        if (isAuthenticated) loadFavoriteState()
    }

    private fun loadFavoriteState() {
        scope.launch {
            favoritesRepository
                .isFavorite(listingId)
                .fold(
                    onSuccess = { fav -> _state.update { it.copy(isFavorite = fav) } },
                    onFailure = { /* non-critical: leave heart in its default state */ },
                )
        }
    }

    private fun loadListing() {
        _state.update { it.copy(isLoading = true, errorMessage = null) }
        scope.launch {
            listingRepository
                .getListingDetail(listingId)
                .fold(
                    onSuccess = { detail ->
                        _state.update { it.copy(listing = detail, isLoading = false) }
                    },
                    onFailure = { e ->
                        _state.update { it.copy(errorMessage = errorMapper(e), isLoading = false) }
                    },
                )
        }
    }

    /** Retry the failed listing load. */
    fun retry() = loadListing()

    /**
     * React to a session change (login / logout) without disturbing the loaded listing.
     *
     * The view-model is keyed only on `listingId` at the composition layer (issue #2108), so an
     * auth transition no longer re-creates it and no longer flips the whole screen back to the
     * full-screen spinner. Instead this rebuilds the auth-scoped repositories with the new token
     * and re-syncs only the favorite heart — leaving `listing` / `isLoading` untouched. A no-op
     * when the token is unchanged, so it is safe to drive from `LaunchedEffect(sessionToken)`
     * (which also fires on first composition).
     */
    fun updateAuth(sessionToken: String?) {
        if (sessionToken == currentSessionToken) return
        currentSessionToken = sessionToken
        isAuthenticated = sessionToken != null
        favoritesRepository = favoritesRepositoryFactory(sessionToken)
        inquiryRepository = inquiryRepositoryFactory(sessionToken)
        if (isAuthenticated) {
            loadFavoriteState()
        } else {
            // Favorites require auth; on logout the heart can no longer reflect a user's state.
            _state.update { it.copy(isFavorite = false, isFavoriteLoading = false) }
        }
    }

    /**
     * Optimistic favorite toggle: flip the heart immediately, then reconcile with the server. On
     * failure the state rolls back to the previous value so the icon never lies about persisted
     * state. No-op when unauthenticated or a toggle is already in flight.
     */
    fun onFavoriteToggle() {
        if (!isAuthenticated) return
        val current = _state.value
        if (current.isFavoriteLoading) return
        val previous = current.isFavorite
        val next = !previous
        _state.update { it.copy(isFavorite = next, isFavoriteLoading = true) }
        scope.launch {
            val result =
                if (next) favoritesRepository.addFavorite(listingId)
                else favoritesRepository.removeFavorite(listingId)
            _state.update { s ->
                if (result.isFailure) s.copy(isFavorite = previous, isFavoriteLoading = false)
                else s.copy(isFavoriteLoading = false)
            }
        }
    }

    fun onShowInquiryDialog() = _state.update { it.copy(showInquiryDialog = true) }

    fun onDismissInquiryDialog() = _state.update {
        it.copy(showInquiryDialog = false, inquiryError = null)
    }

    fun onShowShareSheet() = _state.update { it.copy(showShareSheet = true) }

    fun onDismissShareSheet() = _state.update { it.copy(showShareSheet = false) }

    /**
     * Submit an inquiry for this listing; emits [ListingDetailEvent.InquirySubmitted] on success.
     */
    fun onSubmitInquiry(message: String, name: String?, email: String?, phone: String?) {
        _state.update { it.copy(isInquirySubmitting = true, inquiryError = null) }
        scope.launch {
            val request =
                CreateInquiryRequest(
                    listingId = listingId,
                    message = message,
                    name = name,
                    email = email,
                    phone = phone,
                )
            inquiryRepository
                .createInquiry(request)
                .fold(
                    onSuccess = {
                        _state.update {
                            it.copy(isInquirySubmitting = false, showInquiryDialog = false)
                        }
                        _events.emit(ListingDetailEvent.InquirySubmitted)
                    },
                    onFailure = { e ->
                        _state.update {
                            it.copy(isInquirySubmitting = false, inquiryError = errorMapper(e))
                        }
                    },
                )
        }
    }

    companion object {
        /**
         * Builds the view-model together with its [FavoritesRepository] / [InquiryRepository],
         * keeping repository construction out of the UI layer (previously `remember { … }` inside
         * the composable). `sessionToken == null` is treated as unauthenticated.
         */
        fun create(
            listingId: String,
            listingRepository: ListingRepository,
            baseUrl: String,
            sessionToken: String?,
            scope: CoroutineScope,
            errorMapper: (Throwable) -> String,
        ): ListingDetailViewModel =
            ListingDetailViewModel(
                listingId = listingId,
                listingRepository = listingRepository,
                favoritesRepositoryFactory = { token ->
                    FavoritesRepository(baseUrl = baseUrl, sessionToken = token)
                },
                inquiryRepositoryFactory = { token ->
                    InquiryRepository(baseUrl = baseUrl, sessionToken = token)
                },
                scope = scope,
                initialSessionToken = sessionToken,
                errorMapper = errorMapper,
            )
    }
}
