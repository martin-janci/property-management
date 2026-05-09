package three.two.bit.ppt.reality.ui.realtor

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch

/**
 * Create listing screen for Reality Portal Android (UC-51.4).
 *
 * Captures the minimum fields required to publish a listing. The submit handler is supplied by the
 * caller so the API client integration stays outside the UI layer.
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
            TopAppBar(
                title = { Text("New listing") },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        }
    ) { padding ->
        Column(
            modifier =
                Modifier.fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                text = "Publish a listing",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )

            generalError?.let { Text(it, color = MaterialTheme.colorScheme.error) }

            OutlinedTextField(
                value = title,
                onValueChange = {
                    title = it
                    titleError = null
                },
                label = { Text("Title") },
                singleLine = true,
                isError = titleError != null,
                supportingText = { titleError?.let { Text(it) } },
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = description,
                onValueChange = { description = it },
                label = { Text("Description") },
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = city,
                onValueChange = {
                    city = it
                    cityError = null
                },
                label = { Text("City") },
                singleLine = true,
                isError = cityError != null,
                supportingText = { cityError?.let { Text(it) } },
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = price,
                onValueChange = {
                    price = it.filter { ch -> ch.isDigit() || ch == '.' }
                    priceError = null
                },
                label = { Text("Price") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                isError = priceError != null,
                supportingText = { priceError?.let { Text(it) } },
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = currency,
                onValueChange = { currency = it.uppercase().take(3) },
                label = { Text("Currency") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = transactionType,
                onValueChange = { transactionType = it.lowercase() },
                label = { Text("Transaction (sale or rent)") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            Button(
                onClick = {
                    titleError = if (title.isBlank()) "Title is required" else null
                    cityError = if (city.isBlank()) "City is required" else null
                    val priceNum = price.toDoubleOrNull()
                    priceError =
                        if (priceNum == null || priceNum <= 0) "Enter a positive price" else null
                    if (
                        listOf(titleError, cityError, priceError).any { it != null } ||
                            priceNum == null
                    ) {
                        return@Button
                    }
                    isSubmitting = true
                    generalError = null
                    scope.launch {
                        val result =
                            onSubmit(
                                CreateListingInput(
                                    title = title.trim(),
                                    description = description.trim(),
                                    city = city.trim(),
                                    price = priceNum,
                                    currency = currency,
                                    transactionType = transactionType,
                                )
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
                enabled = !isSubmitting,
                modifier = Modifier.fillMaxWidth(),
                contentPadding = PaddingValues(vertical = 14.dp),
            ) {
                if (isSubmitting) {
                    CircularProgressIndicator(modifier = Modifier.padding(end = 8.dp))
                }
                Text("Publish listing")
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
