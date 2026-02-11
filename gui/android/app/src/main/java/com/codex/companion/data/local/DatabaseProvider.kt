package com.codex.companion.data.local

import android.content.Context
import androidx.room.Room

object DatabaseProvider {
    fun build(context: Context): CompanionDatabase {
        return Room.databaseBuilder(
            context,
            CompanionDatabase::class.java,
            "companion.db",
        )
            .fallbackToDestructiveMigration()
            .build()
    }
}
