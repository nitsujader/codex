package com.codex.companion

import android.content.Context
import com.codex.companion.data.discovery.MdnsDiscovery
import com.codex.companion.data.discovery.StubMdnsDiscovery
import com.codex.companion.data.local.CompanionDatabase
import com.codex.companion.data.local.DatabaseProvider
import com.codex.companion.data.network.CompanionRealtimeClient
import com.codex.companion.data.network.KtorCompanionRealtimeClient
import com.codex.companion.data.repository.CompanionRepository
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import io.ktor.client.plugins.websocket.WebSockets

class AppContainer(context: Context) {
    private val database: CompanionDatabase = DatabaseProvider.build(context)
    private val httpClient: HttpClient = HttpClient(CIO) {
        install(WebSockets) {
            pingInterval = 20_000
        }
    }

    private val mdnsDiscovery: MdnsDiscovery = StubMdnsDiscovery()
    private val realtimeClient: CompanionRealtimeClient = KtorCompanionRealtimeClient(httpClient)

    val repository: CompanionRepository = CompanionRepository(
        pairedDeviceDao = database.pairedDeviceDao(),
        sessionCursorDao = database.sessionCursorDao(),
        realtimeClient = realtimeClient,
        mdnsDiscovery = mdnsDiscovery,
    )
}
