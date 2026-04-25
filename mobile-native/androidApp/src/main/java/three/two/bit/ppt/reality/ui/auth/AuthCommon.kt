package three.two.bit.ppt.reality.ui.auth

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope

/** Shared helpers for the Reality Portal auth screens (UC-47). */
internal val emailRegex = Regex("^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$")

internal const val MIN_PASSWORD_LENGTH = 8

@Composable internal fun rememberAuthScope(): CoroutineScope = rememberCoroutineScope()

@Composable
internal fun ErrorBanner(message: String) {
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .clip(RoundedCornerShape(8.dp))
                .background(Color(0xFFFEF2F2))
                .padding(horizontal = 12.dp, vertical = 10.dp)
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodySmall,
            color = Color(0xFFB91C1C),
        )
    }
}

@Composable
internal fun SuccessBanner(message: String) {
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .clip(RoundedCornerShape(8.dp))
                .background(Color(0xFFECFDF5))
                .padding(horizontal = 12.dp, vertical = 10.dp)
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodySmall,
            color = Color(0xFF047857),
        )
    }
}
