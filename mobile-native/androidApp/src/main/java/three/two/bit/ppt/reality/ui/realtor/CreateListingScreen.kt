package three.two.bit.ppt.reality.ui.realtor

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.ui.auth.AuthFieldLabel
import three.two.bit.ppt.reality.ui.auth.ErrorBanner

/**
 * Create-listing screen — KMP / Compose M3 redesign matching the design
 * (`KmpCreateListingScreen`). Sticky bar with close + "New listing" +
 * "Draft saved" pill; progress bar with 5-step labels (Type / Location /
 * Details / Photos / Price); the screen renders as a single-step
 * "Listing info" pane (the wizard navigation will land when the API
 * supports per-step persistence — for now everything is on one screen).
 * Sticky bottom bar with ghost "Back" + filled "Publish listing".
 *
 * UC-51.4 — Realtor: publish a new listing.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateListingScreen(
    onBackClick: () -> Unit,
    onSubmit: suspend (CreateListingInput) -> Result<Unit>,
    onCreated: () -> Unit,
) {
    var title by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var city by remember { mutableStateOf("") }
    var price by remember { mutableStateOf("") }
    var currency by remember { mutableStateOf("EUR") }
    var transactionType by remember { mutableStateOf("sale") }

    var titleError by remember { mutableStateOf<String?>(null) }
    var priceError by remember { mutableStateOf<String?>(null) }
    var cityError by remember { mutableStateOf<String?>(null) }
    var generalError by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }

    val scope = rememberCoroutineScope()

    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = {
                    Text(
                        text = stringResource(R.string.realtor_create_title),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.Close, contentDescription = stringResource(R.string.close))
                    }
                },
                actions = {
                    DraftSavedPill()
                    Spacer(modifier = Modifier.width(8.dp))
                },
                colors = TopAppBarDefaults.centerAlignedTopAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
        bottomBar = {
            CreateListingFooter(
                onCancel = onBackClick,
                onSubmit = {
                    titleError = if (title.isBlank()) "Title is required" else null
                    cityError = if (city.isBlank()) "City is required" else null
                    val priceNum = price.toDoubleOrNull()
                    priceError = if (priceNum == null || priceNum <= 0)
                        "Enter a positive price" else null
                    if (listOf(titleError, cityError, priceError).any { it != null } ||
                        priceNum == null
                    ) return@CreateListingFooter
                    isSubmitting = true
                    generalError = null
                    scope.launch {
                        val result = onSubmit(
                            CreateListingInput(
                                title = title.trim(),
                                description = description.trim(),
                                city = city.trim(),
                                price = priceNum,
                                currency = currency,
                                transactionType = transactionType,
                            ),
                        )
                        isSubmitting = false
                        result.fold(
                            onSuccess = { onCreated() },
                            onFailure = {
                                generalError = it.message ?: "Could not publish listing."
                            },
                        )
                    }
                },
                isSubmitting = isSubmitting,
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState()),
        ) {
            ProgressHeader()

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 16.dp),
            ) {
                generalError?.let {
                    ErrorBanner(it)
                    Spacer(modifier = Modifier.height(12.dp))
                }

                // Transaction-type segmented control
                AuthFieldLabel(stringResource(R.string.realtor_create_transaction))
                Spacer(modifier = Modifier.height(8.dp))
                Surface(
                    shape = RoundedCornerShape(999.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(modifier = Modifier.padding(3.dp)) {
                        listOf("sale", "rent").forEach { mode ->
                            val isOn = transactionType == mode
                            Surface(
                                onClick = { transactionType = mode },
                                shape = RoundedCornerShape(999.dp),
                                color = if (isOn) MaterialTheme.colorScheme.surface
                                else Color.Transparent,
                                shadowElevation = if (isOn) 1.dp else 0.dp,
                                modifier = Modifier.weight(1f),
                            ) {
                                Text(
                                    text = when (mode) {
                                        "sale" -> stringResource(R.string.for_sale)
                                        else -> stringResource(R.string.for_rent)
                                    },
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(vertical = 10.dp),
                                    style = MaterialTheme.typography.bodyMedium,
                                    fontWeight = FontWeight.SemiBold,
                                    color = if (isOn) MaterialTheme.colorScheme.onSurface
                                    else MaterialTheme.colorScheme.onSurfaceVariant,
                                    textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(18.dp))
                AuthFieldLabel(stringResource(R.string.realtor_create_field_title))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it; titleError = null },
                    placeholder = { Text(stringResource(R.string.realtor_create_title_placeholder)) },
                    singleLine = true,
                    isError = titleError != null,
                    supportingText = { titleError?.let { Text(it) } },
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(14.dp))
                AuthFieldLabel(stringResource(R.string.realtor_create_field_description))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    minLines = 4,
                    maxLines = 8,
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(14.dp))
                AuthFieldLabel(stringResource(R.string.realtor_create_field_city))
                Spacer(modifier = Modifier.height(6.dp))
                OutlinedTextField(
                    value = city,
                    onValueChange = { city = it; cityError = null },
                    singleLine = true,
                    isError = cityError != null,
                    supportingText = { cityError?.let { Text(it) } },
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth(),
                )

                Spacer(modifier = Modifier.height(14.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Column(modifier = Modifier.weight(2f)) {
                        AuthFieldLabel(stringResource(R.string.realtor_create_field_price))
                        Spacer(modifier = Modifier.height(6.dp))
                        OutlinedTextField(
                            value = price,
                            onValueChange = {
                                price = it.filter { ch -> ch.isDigit() || ch == '.' }
                                priceError = null
                            },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                            isError = priceError != null,
                            supportingText = { priceError?.let { Text(it) } },
                            shape = RoundedCornerShape(8.dp),
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                    Column(modifier = Modifier.weight(1f)) {
                        AuthFieldLabel(stringResource(R.string.realtor_create_field_currency))
                        Spacer(modifier = Modifier.height(6.dp))
                        OutlinedTextField(
                            value = currency,
                            onValueChange = { currency = it.uppercase().take(3) },
                            singleLine = true,
                            shape = RoundedCornerShape(8.dp),
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                }

                Spacer(modifier = Modifier.height(110.dp))
            }
        }
    }
}

@Composable
private fun ProgressHeader() {
    Column(modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = stringResource(R.string.realtor_create_step_label),
                style = MaterialTheme.typography.labelSmall.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.04.sp,
                ),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = "100%",
                style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.Bold),
                color = MaterialTheme.colorScheme.primary,
            )
        }
        Spacer(modifier = Modifier.height(8.dp))
        LinearProgressIndicator(
            progress = { 1f },
            modifier = Modifier
                .fillMaxWidth()
                .height(6.dp)
                .clip(RoundedCornerShape(999.dp)),
        )
        Spacer(modifier = Modifier.height(8.dp))
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            listOf(
                R.string.realtor_create_step1,
                R.string.realtor_create_step2,
                R.string.realtor_create_step3,
                R.string.realtor_create_step4,
                R.string.realtor_create_step5,
            ).forEachIndexed { idx, res ->
                Text(
                    text = stringResource(res),
                    style = MaterialTheme.typography.labelSmall.copy(fontSize = 10.sp),
                    fontWeight = if (idx == 0) FontWeight.Bold else FontWeight.Medium,
                    color = if (idx == 0) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun DraftSavedPill() {
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = Color(0xFFD1FAE5),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                Icons.Default.Check,
                contentDescription = null,
                modifier = Modifier.size(10.dp),
                tint = Color(0xFF065F46),
            )
            Spacer(modifier = Modifier.width(4.dp))
            Text(
                text = stringResource(R.string.realtor_create_draft_saved).uppercase(),
                style = MaterialTheme.typography.labelSmall.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.04.sp,
                    fontSize = 10.sp,
                ),
                color = Color(0xFF065F46),
            )
        }
    }
}

@Composable
private fun CreateListingFooter(
    onCancel: () -> Unit,
    onSubmit: () -> Unit,
    isSubmitting: Boolean,
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        shadowElevation = 8.dp,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            OutlinedButton(
                onClick = onCancel,
                modifier = Modifier
                    .weight(1f)
                    .height(48.dp),
                shape = RoundedCornerShape(14.dp),
                enabled = !isSubmitting,
            ) {
                Text(
                    text = stringResource(R.string.back),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
            }
            Button(
                onClick = onSubmit,
                modifier = Modifier
                    .weight(2f)
                    .height(48.dp),
                shape = RoundedCornerShape(14.dp),
                enabled = !isSubmitting,
            ) {
                if (isSubmitting) {
                    CircularProgressIndicator(
                        modifier = Modifier
                            .padding(end = 8.dp)
                            .size(16.dp),
                        strokeWidth = 2.dp,
                        color = MaterialTheme.colorScheme.onPrimary,
                    )
                }
                Text(
                    text = stringResource(R.string.realtor_create_publish),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
            }
        }
    }
}

data class CreateListingInput(
    val title: String,
    val description: String,
    val city: String,
    val price: Double,
    val currency: String,
    val transactionType: String,
)
