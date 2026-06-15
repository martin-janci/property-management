package three.two.bit.ppt.reality.ui.search

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority

/**
 * Resolved device coordinate for the Search "Near Me" radius filter.
 *
 * Epic 82 - Story 82.3: this is the Android counterpart of the iOS `LocationManager`
 * (`mobile-native/iosApp/iosApp/Core/Location/LocationManager.swift`) used by the SwiftUI
 * `FilterSheet.nearMeSection`. Kept intentionally tiny — a single one-shot coordinate fetch is all
 * the radius filter needs; we don't stream location updates.
 */
data class NearMeCoordinate(val latitude: Double, val longitude: Double)

/**
 * A composable handle the FilterSheet uses to turn the "Near Me" toggle on: it owns the runtime
 * permission launcher and the one-shot location request, and reports the resolved coordinate (or a
 * denial) back through [onResolved].
 *
 * Behaviour mirrors the iOS flow:
 * - If coarse-location permission is already granted, fetch the current coordinate immediately.
 * - Otherwise request the permission; on grant, fetch; on denial, report `null` so the caller can
 *   revert the toggle (graceful degradation — the search simply has no location centre).
 */
class NearMeLocationRequester internal constructor(
    private val request: (onResolved: (NearMeCoordinate?) -> Unit) -> Unit,
) {
    /** Begin acquiring the device coordinate; [onResolved] receives null when unavailable/denied. */
    fun acquire(onResolved: (NearMeCoordinate?) -> Unit) = request(onResolved)
}

/**
 * Remembers a [NearMeLocationRequester] bound to the current activity. Must be called from
 * composable scope (it registers an activity-result launcher).
 */
@Composable
fun rememberNearMeLocationRequester(): NearMeLocationRequester {
    val context = LocalContext.current
    // Holds the callback for the in-flight request so the permission-result lambda can complete it.
    val pending = remember { arrayOfNulls<(NearMeCoordinate?) -> Unit>(1) }

    val permissionLauncher =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            val cb = pending[0]
            pending[0] = null
            if (granted) fetchLastLocation(context, cb ?: return@rememberLauncherForActivityResult)
            else cb?.invoke(null)
        }

    return remember {
        NearMeLocationRequester { onResolved ->
            if (hasCoarseLocationPermission(context)) {
                fetchLastLocation(context, onResolved)
            } else {
                pending[0] = onResolved
                permissionLauncher.launch(Manifest.permission.ACCESS_COARSE_LOCATION)
            }
        }
    }
}

private fun hasCoarseLocationPermission(context: Context): Boolean =
    // Framework API (API 23+) — minSdk is 24, so no ContextCompat shim needed.
    context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) ==
        PackageManager.PERMISSION_GRANTED

private fun fetchLastLocation(context: Context, onResolved: (NearMeCoordinate?) -> Unit) {
    val client = LocationServices.getFusedLocationProviderClient(context)
    try {
        // getCurrentLocation triggers a fresh fix when no recent one is cached, unlike
        // lastLocation which can be stale/null on a cold start.
        client
            .getCurrentLocation(Priority.PRIORITY_BALANCED_POWER_ACCURACY, null)
            .addOnSuccessListener { location ->
                onResolved(
                    location?.let { NearMeCoordinate(it.latitude, it.longitude) }
                )
            }
            .addOnFailureListener { onResolved(null) }
    } catch (_: SecurityException) {
        // Permission revoked between the check and the call — degrade gracefully.
        onResolved(null)
    }
}
