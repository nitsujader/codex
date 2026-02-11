package com.codex.companion.model

data class SessionSummary(
    val sessionId: String,
    val title: String,
    val updatedAtLabel: String,
    val unreadCount: Int,
)
