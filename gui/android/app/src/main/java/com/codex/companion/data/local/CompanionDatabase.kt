package com.codex.companion.data.local

import androidx.room.Database
import androidx.room.RoomDatabase
import androidx.room.TypeConverter
import androidx.room.TypeConverters
import com.codex.companion.model.CompanionRole

@Database(
    entities = [
        PairedDeviceEntity::class,
        SessionCursorEntity::class,
    ],
    version = 1,
    exportSchema = false,
)
@TypeConverters(DatabaseConverters::class)
abstract class CompanionDatabase : RoomDatabase() {
    abstract fun pairedDeviceDao(): PairedDeviceDao
    abstract fun sessionCursorDao(): SessionCursorDao
}

class DatabaseConverters {
    @TypeConverter
    fun fromRole(role: CompanionRole): String = role.name

    @TypeConverter
    fun toRole(value: String): CompanionRole = CompanionRole.valueOf(value)
}
