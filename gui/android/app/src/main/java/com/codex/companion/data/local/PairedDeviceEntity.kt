package com.codex.companion.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey
import com.codex.companion.model.CompanionRole

@Entity(tableName = "paired_devices")
data class PairedDeviceEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val host: String,
    val token: String,
    val pairingCode: String,
    val role: CompanionRole,
    val pairedAtEpochSeconds: Long,
)
