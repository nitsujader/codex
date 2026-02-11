package com.codex.companion.data.network

import com.codex.companion.model.ConnectionState
import com.codex.companion.model.ThreadControlAction
import io.ktor.client.HttpClient
import io.ktor.client.plugins.websocket.DefaultClientWebSocketSession
import io.ktor.client.plugins.websocket.send
import io.ktor.client.plugins.websocket.webSocket
import io.ktor.websocket.Frame
import io.ktor.websocket.readText
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class KtorCompanionRealtimeClient(
    private val httpClient: HttpClient,
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO,
) : CompanionRealtimeClient {
    private val scope = CoroutineScope(SupervisorJob() + ioDispatcher)
    private val mutableConnectionState = MutableStateFlow(ConnectionState.DISCONNECTED)

    private var reconnectJob: Job? = null
    private var activeSession: DefaultClientWebSocketSession? = null
    private var running: Boolean = false

    override val connectionState: StateFlow<ConnectionState> = mutableConnectionState.asStateFlow()

    override fun start(config: RealtimeConfig) {
        if (running) {
            return
        }

        running = true
        reconnectJob = scope.launch {
            while (running && currentCoroutineContext().isActive) {
                mutableConnectionState.value = if (mutableConnectionState.value == ConnectionState.DISCONNECTED) {
                    ConnectionState.CONNECTING
                } else {
                    ConnectionState.RECONNECTING
                }

                try {
                    connectOnce(config)
                } catch (_: Throwable) {
                    mutableConnectionState.value = ConnectionState.RECONNECTING
                }

                if (running) {
                    // TODO: Add bounded backoff and jitter before reconnect attempts.
                    delay(config.reconnectDelayMs)
                }
            }
        }
    }

    override fun stop() {
        running = false
        reconnectJob?.cancel()
        reconnectJob = null
        activeSession = null
        mutableConnectionState.value = ConnectionState.DISCONNECTED
    }

    override suspend fun sendPrompt(sessionId: String, prompt: String) {
        val escapedPrompt = prompt.replace("\"", "\\\"")
        activeSession?.send(
            Frame.Text("{\"type\":\"prompt\",\"sessionId\":\"$sessionId\",\"prompt\":\"$escapedPrompt\"}"),
        )
        // TODO: Use structured serialization instead of manual JSON string interpolation.
    }

    override suspend fun sendAction(sessionId: String, action: ThreadControlAction) {
        activeSession?.send(
            Frame.Text("{\"type\":\"action\",\"sessionId\":\"$sessionId\",\"action\":\"$action\"}"),
        )
        // TODO: Align action payload shape with host protocol contract.
    }

    private suspend fun connectOnce(config: RealtimeConfig) {
        httpClient.webSocket(config.webSocketUrl) {
            activeSession = this
            mutableConnectionState.value = ConnectionState.CONNECTED

            // TODO: Send auth handshake using authToken + pairingCode once protocol is defined.
            for (frame in incoming) {
                if (frame is Frame.Text) {
                    val rawText = frame.readText()
                    if (rawText.isEmpty()) {
                        continue
                    }
                    // TODO: Parse inbound host events and route to state observers.
                }
            }
        }

        activeSession = null
        if (running) {
            mutableConnectionState.value = ConnectionState.RECONNECTING
        }
    }
}
