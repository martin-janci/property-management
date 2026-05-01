package three.two.bit.ppt.reality.ui.theme

import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

// ─── Brand palette ───────────────────────────────────────────────────────────
val Brand600 = Color(0xFF2563EB)   // primary
val Brand700 = Color(0xFF1D4ED8)   // primary-hover / dark
val Brand500 = Color(0xFF3B82F6)   // soft accent
val Brand400 = Color(0xFF60A5FA)   // dark-mode primary
val Brand100 = Color(0xFFDBEAFE)
val Brand50  = Color(0xFFEFF6FF)   // primary-soft-bg

// ─── Neutral surface ladder ───────────────────────────────────────────────────
val Neutral50  = Color(0xFFF9FAFB)  // bg-app (light)
val Neutral100 = Color(0xFFF3F4F6)  // bg-subtle
val Neutral200 = Color(0xFFE5E7EB)  // border-default
val Neutral300 = Color(0xFFD1D5DB)  // border-strong
val Neutral400 = Color(0xFF9CA3AF)  // fg-subtle
val Neutral500 = Color(0xFF6B7280)  // fg-muted
val Neutral600 = Color(0xFF4B5563)  // fg-secondary
val Neutral700 = Color(0xFF374151)  // fg-secondary (stronger)
val Neutral800 = Color(0xFF1F2937)
val Neutral900 = Color(0xFF111827)  // fg-primary

// Dark mode surfaces
val Dark100 = Color(0xFF0F1115)  // bg-app
val Dark200 = Color(0xFF1A1D23)  // bg-surface
val Dark300 = Color(0xFF22262E)  // bg-elevated

// ─── Semantic colors ─────────────────────────────────────────────────────────
val DangerRed     = Color(0xFFEF4444)
val DangerRedDark = Color(0xFFDC2626)
val DangerRedBg   = Color(0xFFFEE2E2)
val DangerRedInk  = Color(0xFF991B1B)

val WarningAmber     = Color(0xFFF59E0B)
val WarningAmberDark = Color(0xFFD97706)
val WarningAmberBg   = Color(0xFFFEF3C7)
val WarningAmberInk  = Color(0xFF92400E)

val SuccessGreen     = Color(0xFF10B981)
val SuccessGreenDark = Color(0xFF047857)
val SuccessGreenBg   = Color(0xFFD1FAE5)
val SuccessGreenInk  = Color(0xFF065F46)

val InfoBlue    = Color(0xFF0EA5E9)
val InfoBlueBg  = Color(0xFFE0F2FE)
val InfoBlueInk = Color(0xFF075985)

// ─── Fault status tokens ─────────────────────────────────────────────────────
object FaultStatusColors {
    val newBg        = Color(0xFFFEE2E2);   val newInk        = Color(0xFF991B1B)
    val triagedBg    = Color(0xFFDBEAFE);   val triagedInk    = Color(0xFF1D4ED8)
    val inProgressBg = Color(0xFFEDE9FE);   val inProgressInk = Color(0xFF5B21B6)
    val waitPartsBg  = Color(0xFFFFEDD5);   val waitPartsInk  = Color(0xFF9A3412)
    val scheduledBg  = Color(0xFFEFF6FF);   val scheduledInk  = Color(0xFF1D4ED8)
    val resolvedBg   = Color(0xFFD1FAE5);   val resolvedInk   = Color(0xFF065F46)
    val closedBg     = Color(0xFFF3F4F6);   val closedInk     = Color(0xFF374151)
    val reopenedBg   = Color(0xFFFEF3C7);   val reopenedInk   = Color(0xFF92400E)
}

// ─── Priority tokens ─────────────────────────────────────────────────────────
object PriorityColors {
    val low    = Color(0xFF6B7280)
    val medium = Color(0xFFD97706)
    val high   = Color(0xFFEA580C)
    val urgent = Color(0xFFDC2626)
}

// ─── Badge tokens ────────────────────────────────────────────────────────────
object BadgeColors {
    val saleBg    = Color(0xFF10B981);  val saleInk    = Color.White
    val rentBg    = Color(0xFF3B82F6);  val rentInk    = Color.White
    val featuredBg = Color(0xFFFBBF24); val featuredInk = Color(0xFF78350F)
}

// ─── Material3 color schemes ─────────────────────────────────────────────────
val RealityLightColorScheme = lightColorScheme(
    primary          = Brand600,
    onPrimary        = Color.White,
    primaryContainer = Brand50,
    onPrimaryContainer = Brand700,
    secondary        = Brand500,
    onSecondary      = Color.White,
    background       = Neutral50,
    onBackground     = Neutral900,
    surface          = Color.White,
    onSurface        = Neutral900,
    surfaceVariant   = Neutral100,
    onSurfaceVariant = Neutral600,
    outline          = Neutral200,
    outlineVariant   = Neutral300,
    error            = DangerRed,
    onError          = Color.White,
    errorContainer   = DangerRedBg,
    onErrorContainer = DangerRedInk,
)

val RealityDarkColorScheme = darkColorScheme(
    primary          = Brand400,
    onPrimary        = Dark100,
    primaryContainer = Color(0xFF1E3A8A),
    onPrimaryContainer = Brand400,
    secondary        = Brand500,
    onSecondary      = Dark100,
    background       = Dark100,
    onBackground     = Color(0xFFE5E7EB),
    surface          = Dark200,
    onSurface        = Color(0xFFE5E7EB),
    surfaceVariant   = Dark300,
    onSurfaceVariant = Neutral400,
    outline          = Color(0xFF374151),
    outlineVariant   = Color(0xFF1F2937),
    error            = Color(0xFFF87171),
    onError          = Dark100,
    errorContainer   = Color(0xFF7F1D1D),
    onErrorContainer = Color(0xFFFECACA),
)
