package com.codex.companion.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codex.companion.model.ThreadMessage

@Composable
fun LiveThreadScreen(
    sessionId: String,
    messages: List<ThreadMessage>,
    promptDraft: String,
    innerPadding: PaddingValues,
    onPromptChanged: (String) -> Unit,
    onSendPrompt: () -> Unit,
    onInterrupt: () -> Unit,
    onPause: () -> Unit,
    onResume: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(innerPadding)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("Live Thread: $sessionId")
        LazyColumn(
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(messages, key = { it.id }) { message ->
                Column {
                    Text("${message.author} @ ${message.timestampLabel}")
                    Text(message.content)
                }
            }
        }
        OutlinedTextField(
            value = promptDraft,
            onValueChange = onPromptChanged,
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Prompt") },
            placeholder = { Text("Send instruction to host") },
            minLines = 2,
            maxLines = 4,
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Button(onClick = onSendPrompt, modifier = Modifier.weight(1f)) {
                Text("Send")
            }
            Button(onClick = onInterrupt, modifier = Modifier.weight(1f)) {
                Text("Interrupt")
            }
        }
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Button(onClick = onPause, modifier = Modifier.weight(1f)) {
                Text("Pause")
            }
            Button(onClick = onResume, modifier = Modifier.weight(1f)) {
                Text("Resume")
            }
        }
    }
}
