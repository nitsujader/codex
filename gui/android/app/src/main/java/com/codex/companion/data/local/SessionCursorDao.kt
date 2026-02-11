package com.codex.companion.data.local

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert

@Dao
interface SessionCursorDao {
    @Query("SELECT * FROM session_cursors WHERE sessionId = :sessionId LIMIT 1")
    suspend fun bySessionId(sessionId: String): SessionCursorEntity?

    @Upsert
    suspend fun upsert(cursor: SessionCursorEntity)
}
