package com.codex.companion.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "session_cursors")
data class SessionCursorEntity(
    @PrimaryKey val sessionId: String,
    val cursor: String,
    val updatedAtEpochSeconds: Long,
)
