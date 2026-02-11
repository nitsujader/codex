package com.codex.companion.data.network

import com.codex.companion.model.ConnectionState
import com.codex.companion.model.ThreadControlAction
import kotlinx.coroutines.flow.StateFlow

interface CompanionRealtimeClient {
    val connectionState: StateFlow<ConnectionState>

    fun start(config: RealtimeConfig)

    fun stop()

    suspend fun sendPrompt(sessionId: String, prompt: String)

    suspend fun sendAction(sessionId: String, action: ThreadControlAction)
}

data class RealtimeConfig(
    val webSocketUrl: String,
    val authToken: String,
    val pairingCode: String,
    val reconnectDelayMs: Long = 3_000,
)
