package three.two.bit.ppt.reality.auth

/** What a guarded screen should render based on auth state. */
enum class GuardDecision {
    CONTENT,
    LOADING,
    NOT_SIGNED_IN,
}

/** Single source of truth for the screen-level auth guard decision. */
fun authGuardDecision(state: AuthState): GuardDecision =
    when (state) {
        is AuthState.Authenticated -> GuardDecision.CONTENT
        is AuthState.Loading -> GuardDecision.LOADING
        is AuthState.Unauthenticated, is AuthState.Error -> GuardDecision.NOT_SIGNED_IN
    }
