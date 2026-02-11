package com.codex.companion.data.local

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow

@Dao
interface PairedDeviceDao {
    @Query("SELECT * FROM paired_devices ORDER BY pairedAtEpochSeconds DESC")
    fun observeAll(): Flow<List<PairedDeviceEntity>>

    @Query("SELECT * FROM paired_devices ORDER BY pairedAtEpochSeconds DESC LIMIT 1")
    suspend fun latest(): PairedDeviceEntity?

    @Upsert
    suspend fun upsert(device: PairedDeviceEntity)
}
