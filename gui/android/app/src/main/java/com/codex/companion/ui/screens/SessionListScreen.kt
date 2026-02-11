package com.codex.companion.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codex.companion.model.SessionSummary

@Composable
fun SessionListScreen(
    sessions: List<SessionSummary>,
    innerPadding: PaddingValues,
    onSessionSelected: (String) -> Unit,
) {
    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .padding(innerPadding)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        item {
            Text("Sessions")
        }
        items(sessions, key = { it.sessionId }) { session ->
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { onSessionSelected(session.sessionId) },
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(session.title)
                        Text(session.updatedAtLabel)
                    }
                    if (session.unreadCount > 0) {
                        Text("${session.unreadCount}")
                    }
                }
            }
        }
    }
}
