package com.codex.companion.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codex.companion.viewmodel.CompanionUiState

@Composable
fun PairingScreen(
    uiState: CompanionUiState,
    innerPadding: PaddingValues,
    onHostChanged: (String) -> Unit,
    onTokenChanged: (String) -> Unit,
    onCodeChanged: (String) -> Unit,
    onPairRequested: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(innerPadding)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(text = "Pair with host")
        OutlinedTextField(
            value = uiState.host,
            onValueChange = onHostChanged,
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Host") },
            placeholder = { Text("192.168.1.2:8080") },
            singleLine = true,
        )
        OutlinedTextField(
            value = uiState.token,
            onValueChange = onTokenChanged,
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Token") },
            singleLine = true,
        )
        OutlinedTextField(
            value = uiState.code,
            onValueChange = onCodeChanged,
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Pairing code") },
            singleLine = true,
        )
        Button(
            onClick = onPairRequested,
            modifier = Modifier.fillMaxWidth(),
            enabled = uiState.host.isNotBlank() && uiState.token.isNotBlank() && uiState.code.isNotBlank(),
        ) {
            Text("Pair")
        }
        if (uiState.statusMessage != null) {
            Text(text = uiState.statusMessage)
        }
    }
}
