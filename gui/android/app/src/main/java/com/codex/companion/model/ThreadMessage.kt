package com.codex.companion.model

data class ThreadMessage(
    val id: String,
    val author: String,
    val content: String,
    val timestampLabel: String,
)
