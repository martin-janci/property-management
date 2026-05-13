package three.two.bit.ppt.reality.ui.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.CoroutineScope
import three.two.bit.ppt.reality.ui.theme.Brand500
import three.two.bit.ppt.reality.ui.theme.Brand800
import three.two.bit.ppt.reality.ui.theme.DangerRedBg
import three.two.bit.ppt.reality.ui.theme.DangerRedDark
import three.two.bit.ppt.reality.ui.theme.SuccessGreenBg
import three.two.bit.ppt.reality.ui.theme.SuccessGreenDark

/**
 * Shared helpers for the Reality Portal auth screens (UC-47) — KMP design
 * (`guest-registration-v2-design-system/project/ui_kits/mobile-native/
 * screens-extension.jsx` § Section A).
 */
internal val emailRegex = Regex("^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$")

internal const val MIN_PASSWORD_LENGTH = 8

@Composable internal fun rememberAuthScope(): CoroutineScope = rememberCoroutineScope()

@Composable
internal fun ErrorBanner(message: String) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(8.dp))
            .background(DangerRedBg)
            .padding(horizontal = 12.dp, vertical = 10.dp),
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodySmall,
            color = DangerRedDark,
        )
    }
}

@Composable
internal fun SuccessBanner(message: String) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(8.dp))
            .background(SuccessGreenBg)
            .padding(horizontal = 12.dp, vertical = 10.dp),
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodySmall,
            color = SuccessGreenDark,
        )
    }
}

/**
 * Brand hero used on the Login screen — 56dp gradient square (Brand800 →
 * Brand500) with white "R" wordmark glyph, then "Reality Portal" title and
 * a 1-line muted subtitle. Matches design A1 in screens-extension.jsx.
 */
@Composable
internal fun AuthBrandHero(title: String, subtitle: String) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 56.dp, start = 24.dp, end = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier
                .size(56.dp)
                .clip(RoundedCornerShape(14.dp))
                .drawBehind {
                    drawRect(
                        brush = Brush.linearGradient(
                            colors = listOf(Brand800, Brand500),
                            start = Offset(0f, 0f),
                            end = Offset(size.width, size.height),
                        ),
                    )
                },
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "R",
                color = Color.White,
                fontWeight = FontWeight.Bold,
                fontSize = 22.sp,
            )
        }
        Spacer(modifier = Modifier.height(18.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
            text = subtitle,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/**
 * Page-style auth header used on Register / Forgot / Reset / 2FA — a
 * 72dp tinted square holding a centered icon, then a bold title and a
 * 2-line muted subtitle. Matches the success / icon-led variants in the
 * design (A3 success state, A4 success, A5).
 */
@Composable
internal fun AuthIconHero(
    iconBg: Color,
    iconContent: @Composable () -> Unit,
    title: String,
    subtitle: String,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 32.dp, start = 24.dp, end = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier
                .size(72.dp)
                .clip(RoundedCornerShape(999.dp))
                .background(iconBg),
            contentAlignment = Alignment.Center,
        ) { iconContent() }
        Spacer(modifier = Modifier.height(18.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(10.dp))
        Text(
            text = subtitle,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/**
 * Centered "OR" divider used between primary auth + social-login (skipped
 * here while Google/Apple SSO is not wired). Caller decides whether to
 * render social buttons under it.
 */
@Composable
internal fun OrDivider(label: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        HorizontalDivider(modifier = Modifier.weight(1f), color = MaterialTheme.colorScheme.outline)
        Spacer(modifier = Modifier.width(10.dp))
        Text(
            text = label.uppercase(),
            style = MaterialTheme.typography.labelSmall.copy(
                fontWeight = FontWeight.SemiBold,
                letterSpacing = 0.06.sp,
            ),
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.width(10.dp))
        HorizontalDivider(modifier = Modifier.weight(1f), color = MaterialTheme.colorScheme.outline)
    }
}
