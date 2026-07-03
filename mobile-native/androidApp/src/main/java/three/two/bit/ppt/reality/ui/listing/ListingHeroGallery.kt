package three.two.bit.ppt.reality.ui.listing

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import three.two.bit.ppt.reality.listing.*

// ─── Hero gallery ───────────────────────────────────────────────────────────

@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun HeroGallery(
    images: List<ListingImage>,
    isFavorite: Boolean,
    onBackClick: () -> Unit,
    onShareClick: () -> Unit,
    onFavoriteClick: () -> Unit,
) {
    val pagerState = rememberPagerState(pageCount = { images.size.coerceAtLeast(1) })
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .height(280.dp)
                .background(MaterialTheme.colorScheme.surfaceVariant)
    ) {
        if (images.isEmpty()) {
            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Icon(
                    Icons.Default.Image,
                    contentDescription = null,
                    modifier = Modifier.size(64.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            HorizontalPager(state = pagerState, modifier = Modifier.fillMaxSize()) { page ->
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(images[page].url)
                            .crossfade(true)
                            .build(),
                    contentDescription = images[page].caption,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )
            }
        }

        // Back pill (top-left)
        WhitePill(
            modifier = Modifier.align(Alignment.TopStart).padding(14.dp),
            icon = Icons.AutoMirrored.Filled.ArrowBack,
            tint = MaterialTheme.colorScheme.onSurface,
            onClick = onBackClick,
        )

        // Share + heart pills (top-right)
        Row(
            modifier = Modifier.align(Alignment.TopEnd).padding(14.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            WhitePill(
                icon = Icons.Default.Share,
                tint = MaterialTheme.colorScheme.onSurface,
                onClick = onShareClick,
            )
            WhitePill(
                icon = if (isFavorite) Icons.Default.Favorite else Icons.Default.FavoriteBorder,
                tint =
                    if (isFavorite) MaterialTheme.colorScheme.error
                    else MaterialTheme.colorScheme.onSurface,
                onClick = onFavoriteClick,
            )
        }

        if (images.size > 1) {
            // Page dots
            Row(
                modifier = Modifier.align(Alignment.BottomCenter).padding(14.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                repeat(images.size) { idx ->
                    Box(
                        modifier =
                            Modifier.size(6.dp)
                                .clip(CircleShape)
                                .background(
                                    if (idx == pagerState.currentPage) Color.White
                                    else Color.White.copy(alpha = 0.5f)
                                )
                    )
                }
            }
            // Counter (bottom-right)
            Surface(
                modifier = Modifier.align(Alignment.BottomEnd).padding(14.dp),
                shape = RoundedCornerShape(999.dp),
                color = Color.Black.copy(alpha = 0.6f),
            ) {
                Text(
                    text = "${pagerState.currentPage + 1} / ${images.size}",
                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                    style =
                        MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold),
                    color = Color.White,
                )
            }
        }
    }
}

@Composable
private fun WhitePill(
    modifier: Modifier = Modifier,
    icon: ImageVector,
    tint: Color,
    onClick: () -> Unit,
) {
    Box(
        modifier =
            modifier
                .size(40.dp)
                .clip(CircleShape)
                .background(Color.White.copy(alpha = 0.92f))
                .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(18.dp),
            tint = tint,
        )
    }
}
