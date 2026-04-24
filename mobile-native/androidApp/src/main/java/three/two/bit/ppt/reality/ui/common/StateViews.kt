package three.two.bit.ppt.reality.ui.common

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

/**
 * Shared empty/error/loading composables for the Reality Portal Android UI.
 *
 * These mirror the React-side primitives so every screen has consistent
 * empty/loading/error treatment without each one rolling its own.
 */

@Composable
fun EmptyState(
    title: String,
    description: String? = null,
    icon: String = "📭",
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(text = icon, style = MaterialTheme.typography.displaySmall)
        Spacer(Modifier.height(12.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            textAlign = TextAlign.Center,
        )
        if (description != null) {
            Spacer(Modifier.height(8.dp))
            Text(
                text = description,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
fun ErrorState(
    title: String = "Something went wrong",
    description: String? = "Please try again. If the issue persists, contact support.",
    icon: String = "⚠️",
    onRetry: (() -> Unit)? = null,
    retryLabel: String = "Try again",
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(text = icon, style = MaterialTheme.typography.displaySmall)
        Spacer(Modifier.height(12.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            textAlign = TextAlign.Center,
        )
        if (description != null) {
            Spacer(Modifier.height(8.dp))
            Text(
                text = description,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
        if (onRetry != null) {
            Spacer(Modifier.height(16.dp))
            Button(
                onClick = onRetry,
                contentPadding = PaddingValues(horizontal = 24.dp, vertical = 12.dp),
                colors = ButtonDefaults.buttonColors(),
            ) {
                Text(retryLabel)
            }
        }
    }
}

/**
 * Skeleton placeholder block. Animates a soft shimmer between two surface
 * tones; honours the platform's reduce-motion preference automatically via
 * Compose's animation specs.
 */
@Composable
fun SkeletonBlock(
    modifier: Modifier = Modifier,
    height: Dp = 16.dp,
    cornerRadius: Dp = 8.dp,
) {
    var toggled by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        while (true) {
            toggled = !toggled
            delay(750)
        }
    }
    val baseColor = MaterialTheme.colorScheme.surfaceVariant
    val highlightColor = MaterialTheme.colorScheme.surface
    val progress by
        animateFloatAsState(
            targetValue = if (toggled) 1f else 0f,
            animationSpec = tween(durationMillis = 750),
            label = "skeleton-shimmer",
        )
    val color: Color = lerp(baseColor, highlightColor, progress)
    Box(
        modifier =
            modifier
                .fillMaxWidth()
                .height(height)
                .clip(RoundedCornerShape(cornerRadius))
                .background(color)
    )
}

/** Stack of `count` skeleton card placeholders. */
@Composable
fun SkeletonCardList(count: Int, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        repeat(count) { SkeletonBlock(height = 96.dp, cornerRadius = 12.dp) }
    }
}
