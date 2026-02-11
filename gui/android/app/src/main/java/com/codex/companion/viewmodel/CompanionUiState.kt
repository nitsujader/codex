package com.codex.companion.viewmodel

import com.codex.companion.model.CompanionRole
import com.codex.companion.model.ConnectionState
import com.codex.companion.model.SessionSummary
import com.codex.companion.model.ThreadMessage

data class CompanionUiState(
    val host: String = "",
    val token: String = "",
    val code: String = "",
    val isPaired: Boolean = false,
    val pairedHost: String? = null,
    val statusMessage: String? = null,
    val connectionState: ConnectionState = ConnectionState.DISCONNECTED,
    val role: CompanionRole = CompanionRole.CONTROLLER,
    val sessions: List<SessionSummary> = emptyList(),
    val currentThreadMessages: List<ThreadMessage> = emptyList(),
    val promptDraft: String = "",
)
