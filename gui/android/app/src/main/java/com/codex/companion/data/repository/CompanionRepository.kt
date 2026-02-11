package com.codex.companion.data.repository

import com.codex.companion.data.discovery.DiscoveredPeer
import com.codex.companion.data.discovery.MdnsDiscovery
import com.codex.companion.data.local.PairedDeviceDao
import com.codex.companion.data.local.PairedDeviceEntity
import com.codex.companion.data.local.SessionCursorDao
import com.codex.companion.data.local.SessionCursorEntity
import com.codex.companion.data.network.CompanionRealtimeClient
import com.codex.companion.data.network.RealtimeConfig
import com.codex.companion.model.CompanionRole
import com.codex.companion.model.ConnectionState
import com.codex.companion.model.SessionSummary
import com.codex.companion.model.ThreadControlAction
import com.codex.companion.model.ThreadMessage
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.flowOf

class CompanionRepository(
    private val pairedDeviceDao: PairedDeviceDao,
    private val sessionCursorDao: SessionCursorDao,
    private val realtimeClient: CompanionRealtimeClient,
    private val mdnsDiscovery: MdnsDiscovery,
) {
    private val mutableRole = MutableStateFlow(CompanionRole.CONTROLLER)

    val role: StateFlow<CompanionRole> = mutableRole.asStateFlow()
    val connectionState: StateFlow<ConnectionState> = realtimeClient.connectionState

    fun discoveredPeers(): Flow<List<DiscoveredPeer>> = mdnsDiscovery.discoverPeers()

    fun sessions(): Flow<List<SessionSummary>> {
        return flowOf(
            listOf(
                SessionSummary(
                    sessionId = "session-1",
                    title = "Latest Local Thread",
                    updatedAtLabel = "Updated just now",
                    unreadCount = 0,
                ),
                SessionSummary(
                    sessionId = "session-2",
                    title = "Build Agent Debugging",
                    updatedAtLabel = "Updated 4m ago",
                    unreadCount = 2,
                ),
            ),
        )
    }

    fun threadMessages(sessionId: String): Flow<List<ThreadMessage>> {
        return flowOf(
            listOf(
                ThreadMessage(
                    id = "msg-$sessionId-system",
                    author = "system",
                    content = "Live thread placeholder for $sessionId",
                    timestampLabel = "now",
                ),
            ),
        )
    }

    suspend fun pairDevice(host: String, token: String, code: String, role: CompanionRole) {
        pairedDeviceDao.upsert(
            PairedDeviceEntity(
                host = host,
                token = token,
                pairingCode = code,
                role = role,
                pairedAtEpochSeconds = System.currentTimeMillis() / 1000,
            ),
        )

        mutableRole.value = role

        realtimeClient.start(
            RealtimeConfig(
                webSocketUrl = buildWebSocketUrl(host),
                authToken = token,
                pairingCode = code,
            ),
        )
    }

    suspend fun loadLatestPairing(): PairedDeviceEntity? {
        val device = pairedDeviceDao.latest() ?: return null
        mutableRole.value = device.role
        return device
    }

    suspend fun sendPrompt(sessionId: String, prompt: String) {
        realtimeClient.sendPrompt(sessionId, prompt)
    }

    suspend fun sendAction(sessionId: String, action: ThreadControlAction) {
        realtimeClient.sendAction(sessionId, action)
    }

    suspend fun saveSessionCursor(sessionId: String, cursor: String) {
        sessionCursorDao.upsert(
            SessionCursorEntity(
                sessionId = sessionId,
                cursor = cursor,
                updatedAtEpochSeconds = System.currentTimeMillis() / 1000,
            ),
        )
    }

    private fun buildWebSocketUrl(host: String): String {
        return if (host.startsWith("ws://") || host.startsWith("wss://")) {
            host
        } else {
            "ws://$host/ws"
        }
    }
}
