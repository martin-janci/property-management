package three.two.bit.ppt.reality.ui.realtor

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import three.two.bit.ppt.reality.R

/**
 * Listing analytics screen — KMP / Compose M3 redesign matching the
 * design (`KmpListingAnalyticsScreen`). Sticky bar with back + share
 * action; 2-col metric grid where each card has a label, large value,
 * delta, and a 32dp sparkline drawn from the optional `trend` list.
 *
 * UC-51.10 — Realtor analytics.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ListingAnalyticsScreen(
    metrics: List<AnalyticsMetric>,
    isLoading: Boolean,
    onBackClick: () -> Unit,
) {
    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = {
                    Text(
                        text = stringResource(R.string.realtor_analytics_title),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                actions = {
                    IconButton(onClick = { /* share */ }) {
                        Icon(Icons.Default.Share, contentDescription = stringResource(R.string.share))
                    }
                },
                colors = TopAppBarDefaults.centerAlignedTopAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
    ) { padding ->
        if (isLoading) {
            Box(
                modifier = Modifier.fillMaxSize().padding(padding),
                contentAlignment = Alignment.Center,
            ) { CircularProgressIndicator() }
            return@Scaffold
        }
        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            items(items = metrics, key = { it.label }) { metric ->
                MetricCard(metric = metric)
            }
        }
    }
}

@Composable
private fun MetricCard(metric: AnalyticsMetric) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Text(
                text = metric.label,
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(6.dp))
            Row(verticalAlignment = Alignment.Bottom) {
                Text(
                    text = metric.value,
                    style = MaterialTheme.typography.headlineMedium.copy(fontSize = 26.sp),
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                metric.delta?.let {
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = it,
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.SemiBold,
                        color = if (it.startsWith("-")) Color(0xFFEF4444) else Color(0xFF10B981),
                    )
                }
            }
            if (metric.trend.isNotEmpty()) {
                Spacer(modifier = Modifier.height(10.dp))
                Sparkline(values = metric.trend, color = MaterialTheme.colorScheme.primary)
            }
        }
    }
}

@Composable
private fun Sparkline(values: List<Int>, color: Color) {
    if (values.isEmpty()) return
    val maxVal = (values.max().toFloat()).coerceAtLeast(1f)
    Canvas(
        modifier = Modifier
            .fillMaxWidth()
            .height(32.dp),
    ) {
        val w = size.width
        val h = size.height
        val stepX = if (values.size > 1) w / (values.size - 1) else w
        val path = Path()
        val fillPath = Path()
        values.forEachIndexed { i, v ->
            val x = stepX * i
            val y = h - (v / maxVal) * h
            if (i == 0) {
                path.moveTo(x, y)
                fillPath.moveTo(x, h)
                fillPath.lineTo(x, y)
            } else {
                path.lineTo(x, y)
                fillPath.lineTo(x, y)
            }
        }
        fillPath.lineTo(w, h)
        fillPath.close()
        drawPath(
            path = fillPath,
            color = color.copy(alpha = 0.12f),
        )
        drawPath(
            path = path,
            color = color,
            style = Stroke(width = 2.dp.toPx()),
        )
    }
}

/** Lightweight VM-shaped data class fed into the screen. */
data class AnalyticsMetric(
    val label: String,
    val value: String,
    val delta: String? = null,
    val trend: List<Int> = emptyList(),
)
