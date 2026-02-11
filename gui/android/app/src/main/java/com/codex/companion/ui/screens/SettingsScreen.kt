package com.codex.companion.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codex.companion.model.CompanionRole
import com.codex.companion.model.ConnectionState

@Composable
fun SettingsScreen(
    connectionState: ConnectionState,
    role: CompanionRole,
    pairedHost: String?,
    innerPadding: PaddingValues,
    onRoleChanged: (CompanionRole) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(innerPadding)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Settings")
        Text("Connection: $connectionState")
        Text("Role: $role")
        Text("Paired host: ${pairedHost ?: "Not paired"}")

        Text("Select role")
        RoleOption(
            role = CompanionRole.CONTROLLER,
            selectedRole = role,
            onSelected = onRoleChanged,
        )
        RoleOption(
            role = CompanionRole.VIEWER,
            selectedRole = role,
            onSelected = onRoleChanged,
        )
    }
}

@Composable
private fun RoleOption(
    role: CompanionRole,
    selectedRole: CompanionRole,
    onSelected: (CompanionRole) -> Unit,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        RadioButton(
            selected = selectedRole == role,
            onClick = { onSelected(role) },
        )
        Text(role.name)
    }
}
