package com.codex.companion.data.network

import com.codex.companion.model.ConnectionState
import com.codex.companion.model.SessionSummary
import com.codex.companion.model.ThreadControlAction
import com.codex.companion.model.ThreadMessage
import io.ktor.client.HttpClient
import io.ktor.client.plugins.websocket.DefaultClientWebSocketSession
import io.ktor.client.plugins.websocket.webSocket
import io.ktor.http.HttpHeaders
import io.ktor.websocket.Frame
import io.ktor.websocket.readText
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeout
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.atomic.AtomicLong
import kotlin.random.Random

class KtorCompanionRealtimeClient(
    private val httpClient: HttpClient,
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO,
) : CompanionRealtimeClient {
    private val scope = CoroutineScope(SupervisorJob() + ioDispatcher)
    private val mutableConnectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    private val mutableSessions = MutableStateFlow(
        listOf(
            SessionSummary(
                sessionId = HUB_SESSION_ID,
                title = "Hub Events",
                updatedAtLabel = "Not connected",
                unreadCount = 0,
            ),
        ),
    )
    private val mutableThreadMessages =
        MutableStateFlow<Map<String, List<ThreadMessage>>>(mapOf(HUB_SESSION_ID to emptyList()))

    private val pendingRequests = mutableMapOf<String, CompletableDeferred<JSONObject>>()
    private val pendingRequestMutex = Mutex()
    private val requestIdCounter = AtomicLong(1)
    private val latestTurnIdByThread = mutableMapOf<String, String>()

    private var reconnectJob: Job? = null
    private var activeSession: DefaultClientWebSocketSession? = null
    private var running: Boolean = false
    private var defaultThreadId: String? = null

    override val connectionState: StateFlow<ConnectionState> = mutableConnectionState.asStateFlow()
    override val sessions: StateFlow<List<SessionSummary>> = mutableSessions.asStateFlow()
    override val threadMessages: Flow<Map<String, List<ThreadMessage>>> = mutableThreadMessages.asStateFlow()

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
                } catch (error: Throwable) {
                    mutableConnectionState.value = ConnectionState.RECONNECTING
                    appendMessage(
                        sessionId = HUB_SESSION_ID,
                        author = "system",
                        content = "Realtime connection failed: ${error.message ?: "unknown error"}",
                    )
                }

                if (running) {
                    val jitterMs = Random.nextLong(from = 0, until = 750)
                    delay(config.reconnectDelayMs + jitterMs)
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
        scope.launch {
            pendingRequestMutex.withLock {
                for ((_, deferred) in pendingRequests) {
                    deferred.cancel()
                }
                pendingRequests.clear()
            }
        }
    }

    override suspend fun sendPrompt(sessionId: String, prompt: String) {
        val session = activeSession ?: return
        val threadId = resolveThreadIdForPrompt(session, sessionId)
        val params = JSONObject()
            .put("threadId", threadId)
            .put(
                "input",
                JSONArray().put(
                    JSONObject()
                        .put("type", "text")
                        .put("text", prompt)
                        .put("textElements", JSONArray()),
                ),
            )
        rpcRequest(session, "turn/start", params)
        appendMessage(
            sessionId = threadId,
            author = "you",
            content = prompt,
        )
    }

    override suspend fun sendAction(sessionId: String, action: ThreadControlAction) {
        val session = activeSession ?: return
        val threadId = if (sessionId == HUB_SESSION_ID) defaultThreadId else sessionId
        if (threadId.isNullOrBlank()) {
            appendMessage(HUB_SESSION_ID, "system", "No active thread available for action.")
            return
        }

        when (action) {
            ThreadControlAction.INTERRUPT,
            ThreadControlAction.PAUSE -> {
                val params = JSONObject()
                    .put("threadId", threadId)
                    .put("turnId", latestTurnIdByThread[threadId] ?: JSONObject.NULL)
                rpcRequest(session, "agent/interrupt", params)
                appendMessage(threadId, "system", "Interrupt requested.")
            }
            ThreadControlAction.RESUME -> {
                val latestTurn = latestTurnIdByThread[threadId]
                if (!latestTurn.isNullOrBlank()) {
                    val params = JSONObject()
                        .put("threadId", threadId)
                        .put(
                            "input",
                            JSONArray().put(
                                JSONObject()
                                    .put("type", "text")
                                    .put("text", "Continue with the interrupted task.")
                                    .put("textElements", JSONArray()),
                            ),
                        )
                        .put("expectedTurnId", latestTurn)
                    rpcRequest(session, "turn/steer", params)
                    appendMessage(threadId, "system", "Resume steer sent.")
                } else {
                    val params = JSONObject()
                        .put("threadId", threadId)
                        .put(
                            "input",
                            JSONArray().put(
                                JSONObject()
                                    .put("type", "text")
                                    .put("text", "Continue with the previous task.")
                                    .put("textElements", JSONArray()),
                            ),
                        )
                    rpcRequest(session, "turn/start", params)
                    appendMessage(threadId, "system", "Resume prompt queued.")
                }
            }
        }
    }

    private suspend fun resolveThreadIdForPrompt(
        session: DefaultClientWebSocketSession,
        requestedSessionId: String,
    ): String {
        if (requestedSessionId != HUB_SESSION_ID) {
            return requestedSessionId
        }

        if (!defaultThreadId.isNullOrBlank()) {
            return defaultThreadId!!
        }

        val response = rpcRequest(
            session,
            "thread/start",
            JSONObject()
                .put("model", JSONObject.NULL)
                .put("modelProvider", JSONObject.NULL)
                .put("cwd", JSONObject.NULL)
                .put("approvalPolicy", JSONObject.NULL)
                .put("sandbox", JSONObject.NULL)
                .put("config", JSONObject.NULL)
                .put("baseInstructions", JSONObject.NULL)
                .put("developerInstructions", JSONObject.NULL)
                .put("personality", JSONObject.NULL)
                .put("ephemeral", JSONObject.NULL),
        )
        val thread = response.optJSONObject("thread") ?: JSONObject()
        val threadId = thread.optString("id")
        if (threadId.isBlank()) {
            throw IllegalStateException("thread/start did not return a thread id.")
        }
        defaultThreadId = threadId
        upsertSession(
            sessionId = threadId,
            title = thread.optString("preview", "Thread ${threadId.take(8)}"),
            updatedAtLabel = formatUnixTime(System.currentTimeMillis() / 1000),
        )
        return threadId
    }

    private suspend fun connectOnce(config: RealtimeConfig) {
        httpClient.webSocket(
            urlString = websocketUrlWithToken(config.webSocketUrl, config.authToken),
            request = {
                headers.append(HttpHeaders.Authorization, "Bearer ")
            },
        ) {
            activeSession = this
            mutableConnectionState.value = ConnectionState.CONNECTED

            val inboundFramesJob = launch {
                for (frame in incoming) {
                    if (frame is Frame.Text) {
                        val rawText = frame.readText()
                        if (rawText.isNotBlank()) {
                            handleInboundMessage(rawText)
                        }
                    }
                }
            }

            try {
                initializeSession(this)
                refreshSnapshot(this)
                appendMessage(
                    sessionId = HUB_SESSION_ID,
                    author = "system",
                    content = "Connected to ",
                )
                inboundFramesJob.join()
            } finally {
                inboundFramesJob.cancel()
            }
        }

        activeSession = null
        if (running) {
            mutableConnectionState.value = ConnectionState.RECONNECTING
        }
    }

    private suspend fun initializeSession(session: DefaultClientWebSocketSession) {
        try {
            rpcRequest(
                session,
                "initialize",
                JSONObject()
                    .put(
                        "clientInfo",
                        JSONObject()
                            .put("name", "codex_android_companion")
                            .put("title", "Codex Android Companion")
                            .put("version", "0.1.0"),
                    )
                    .put(
                        "capabilities",
                        JSONObject()
                            .put("experimentalApi", false)
                            .put("optOutNotificationMethods", JSONArray()),
                    ),
            )
        } catch (error: IllegalStateException) {
            val message = error.message.orEmpty()
            if (!message.contains("already initialized", ignoreCase = true)) {
                throw error
            }
        }

        session.outgoing.send(
            Frame.Text(
                JSONObject()
                    .put("jsonrpc", "2.0")
                    .put("method", "initialized")
                    .toString(),
            ),
        )
    }

    private suspend fun refreshSnapshot(session: DefaultClientWebSocketSession) {
        val threadsResponse = rpcRequest(
            session,
            "thread/list",
            JSONObject()
                .put("cursor", JSONObject.NULL)
                .put("limit", 100)
                .put("archived", false),
        )
        val threadArray = threadsResponse.optJSONArray("data") ?: JSONArray()
        val summaries = mutableListOf(
            SessionSummary(
                sessionId = HUB_SESSION_ID,
                title = "Hub Events",
                updatedAtLabel = formatUnixTime(System.currentTimeMillis() / 1000),
                unreadCount = 0,
            ),
        )
        for (index in 0 until threadArray.length()) {
            val thread = threadArray.optJSONObject(index) ?: continue
            val threadId = thread.optString("id")
            if (threadId.isBlank()) {
                continue
            }
            val preview = thread.optString("preview", "Thread ${threadId.take(8)}")
            val updatedAt = thread.optLong("updatedAt", System.currentTimeMillis() / 1000)
            summaries += SessionSummary(
                sessionId = threadId,
                title = if (preview.isBlank()) "Thread ${threadId.take(8)}" else preview,
                updatedAtLabel = formatUnixTime(updatedAt),
                unreadCount = 0,
            )
            if (!mutableThreadMessages.value.containsKey(threadId)) {
                mutableThreadMessages.value = mutableThreadMessages.value
                    .toMutableMap()
                    .also { map -> map[threadId] = emptyList() }
                    .toMap()
            }
            if (defaultThreadId.isNullOrBlank()) {
                defaultThreadId = threadId
            }
        }
        mutableSessions.value = summaries

        val deviceResponse = rpcRequest(
            session,
            "device/list",
            JSONObject()
                .put("cursor", JSONObject.NULL)
                .put("limit", 200),
        )
        val devices = deviceResponse.optJSONArray("data") ?: JSONArray()
        appendMessage(HUB_SESSION_ID, "hub", "Paired devices: ${devices.length()}.")
    }

    private suspend fun rpcRequest(
        session: DefaultClientWebSocketSession,
        method: String,
        params: JSONObject,
    ): JSONObject {
        val requestId = requestIdCounter.getAndIncrement().toString()
        val deferred = CompletableDeferred<JSONObject>()
        pendingRequestMutex.withLock {
            pendingRequests[requestId] = deferred
        }

        val payload = JSONObject()
            .put("jsonrpc", "2.0")
            .put("id", requestId)
            .put("method", method)
            .put("params", params)
            .toString()
        session.outgoing.send(Frame.Text(payload))

        return try {
            withTimeout(RPC_TIMEOUT_MS) {
                deferred.await()
            }
        } finally {
            pendingRequestMutex.withLock {
                pendingRequests.remove(requestId)
            }
        }
    }

    private suspend fun handleInboundMessage(rawText: String) {
        val envelope = runCatching { JSONObject(rawText) }.getOrNull() ?: return

        if (envelope.has("id") && (envelope.has("result") || envelope.has("error"))) {
            val requestId = envelope.opt("id")?.toString() ?: return
            val pending = pendingRequestMutex.withLock {
                pendingRequests.remove(requestId)
            } ?: return

            if (envelope.has("error")) {
                val errorObject = envelope.optJSONObject("error")
                val message = errorObject?.optString("message")
                    ?: "Unknown JSON-RPC error."
                pending.completeExceptionally(IllegalStateException(message))
            } else {
                val result = envelope.opt("result")
                val resultObject = if (result is JSONObject) {
                    result
                } else {
                    JSONObject().put("value", result)
                }
                pending.complete(resultObject)
            }
            return
        }

        if (envelope.has("method")) {
            handleRpcNotification(envelope.optString("method"), envelope.opt("params"))
            return
        }

        if (envelope.has("event")) {
            handleLegacyHubEvent(envelope)
        }
    }

    private fun handleRpcNotification(method: String, params: Any?) {
        val payload = params as? JSONObject ?: JSONObject()
        val at = System.currentTimeMillis() / 1000
        when (method) {
            "thread/started" -> {
                val thread = payload.optJSONObject("thread") ?: JSONObject()
                val threadId = thread.optString("id")
                if (threadId.isNotBlank()) {
                    val preview = thread.optString("preview", "Thread ${threadId.take(8)}")
                    upsertSession(threadId, preview, formatUnixTime(at))
                    if (defaultThreadId.isNullOrBlank()) {
                        defaultThreadId = threadId
                    }
                    appendMessage(threadId, "hub", "Thread started.", at)
                }
            }
            "thread/name/updated" -> {
                val threadId = payload.optString("threadId")
                val name = payload.optString("threadName", "Thread ${threadId.take(8)}")
                if (threadId.isNotBlank()) {
                    upsertSession(threadId, name, formatUnixTime(at))
                    appendMessage(threadId, "hub", "Thread renamed to \"$name\".", at)
                }
            }
            "turn/started" -> {
                val threadId = payload.optString("threadId")
                val turn = payload.optJSONObject("turn")
                val turnId = turn?.optString("id")
                if (threadId.isNotBlank() && !turnId.isNullOrBlank()) {
                    latestTurnIdByThread[threadId] = turnId
                }
                if (threadId.isNotBlank()) {
                    appendMessage(threadId, "hub", "Turn started.", at)
                }
            }
            "turn/completed" -> {
                val threadId = payload.optString("threadId")
                if (threadId.isNotBlank()) {
                    latestTurnIdByThread.remove(threadId)
                    appendMessage(threadId, "hub", "Turn completed.", at)
                }
            }
            "item/agentMessage/delta" -> {
                val threadId = payload.optString("threadId")
                val delta = payload.optString("delta")
                if (threadId.isNotBlank() && delta.isNotBlank()) {
                    appendMessage(threadId, "agent", delta, at)
                }
            }
            "item/completed" -> {
                val threadId = payload.optString("threadId")
                val item = payload.optJSONObject("item") ?: JSONObject()
                val type = item.optString("type")
                if (threadId.isNotBlank() && type == "agentMessage") {
                    val text = item.optString("text")
                    if (text.isNotBlank()) {
                        appendMessage(threadId, "agent", text, at)
                    }
                }
            }
            "thread/pinnedPromptUpdated" -> {
                val threadId = payload.optString("threadId")
                if (threadId.isNotBlank()) {
                    appendMessage(threadId, "hub", "Pinned prompt updated.", at)
                }
            }
            "thread/compacted" -> {
                val threadId = payload.optString("threadId")
                if (threadId.isNotBlank()) {
                    appendMessage(threadId, "hub", "Thread compacted.", at)
                }
            }
            "stream/updated" -> {
                val threadId = payload.optString("threadId")
                val streamPath = payload.optString("streamPath")
                if (threadId.isNotBlank()) {
                    appendMessage(threadId, "hub", "Stream updated: $streamPath", at)
                }
            }
            "agent/updated" -> {
                val threadId = payload.optString("threadId")
                val status = payload.optString("status", "updated")
                val message = payload.optString("message")
                if (threadId.isNotBlank()) {
                    val text = if (message.isNullOrBlank()) {
                        "Agent status: $status"
                    } else {
                        "Agent status: $status ($message)"
                    }
                    appendMessage(threadId, "hub", text, at)
                }
            }
            "device/pairingRequested" -> {
                val code = payload.optString("code", "<missing>")
                appendMessage(HUB_SESSION_ID, "hub", "Pairing requested with code $code.", at)
            }
            else -> {
                appendMessage(HUB_SESSION_ID, "hub", "Notification: $method", at)
            }
        }
    }

    private fun handleLegacyHubEvent(eventEnvelope: JSONObject) {
        val event = eventEnvelope.optString("event")
        val at = eventEnvelope.optLong("at", System.currentTimeMillis() / 1000)
        val payload = eventEnvelope.optJSONObject("payload") ?: JSONObject()
        when (event) {
            "hub.connected" -> appendMessage(HUB_SESSION_ID, "hub", "Hub stream connected.", at)
            "device.paired" -> {
                val deviceName = payload.optString("deviceName", "device")
                appendMessage(HUB_SESSION_ID, "hub", "Device paired: $deviceName", at)
            }
            "device.revoked" -> {
                val deviceId = payload.optString("deviceId", "unknown")
                appendMessage(HUB_SESSION_ID, "hub", "Device revoked: $deviceId", at)
            }
            else -> appendMessage(HUB_SESSION_ID, "hub", "Event: $event", at)
        }
    }

    private fun upsertSession(sessionId: String, title: String, updatedAtLabel: String) {
        val existing = mutableSessions.value.filterNot { it.sessionId == sessionId }
        val next = SessionSummary(
            sessionId = sessionId,
            title = title,
            updatedAtLabel = updatedAtLabel,
            unreadCount = 0,
        )
        mutableSessions.value = listOf(next) + existing
        if (!mutableThreadMessages.value.containsKey(sessionId)) {
            mutableThreadMessages.value = mutableThreadMessages.value
                .toMutableMap()
                .also { map -> map[sessionId] = emptyList() }
                .toMap()
        }
    }

    private fun appendMessage(
        sessionId: String,
        author: String,
        content: String,
        atEpochSeconds: Long = System.currentTimeMillis() / 1000,
    ) {
        val currentMessages = mutableThreadMessages.value.toMutableMap()
        val history = currentMessages[sessionId].orEmpty()
        val message = ThreadMessage(
            id = "msg-${sessionId}-${atEpochSeconds}-${history.size}",
            author = author,
            content = content,
            timestampLabel = formatUnixTime(atEpochSeconds),
        )
        currentMessages[sessionId] = (history + message).takeLast(MESSAGE_BUFFER_LIMIT)
        mutableThreadMessages.value = currentMessages.toMap()

        val currentTitle = mutableSessions.value
            .firstOrNull { it.sessionId == sessionId }
            ?.title
            ?: if (sessionId == HUB_SESSION_ID) "Hub Events" else "Thread ${sessionId.take(8)}"
        upsertSession(
            sessionId = sessionId,
            title = currentTitle,
            updatedAtLabel = message.timestampLabel,
        )
    }

    private fun formatUnixTime(unixSeconds: Long): String {
        val millis = unixSeconds * 1000
        return java.text.SimpleDateFormat("HH:mm", java.util.Locale.getDefault())
            .format(java.util.Date(millis))
    }

    private fun websocketUrlWithToken(baseUrl: String, token: String): String {
        if (token.isBlank() || baseUrl.contains("token=")) {
            return baseUrl
        }
        val separator = if (baseUrl.contains("?")) {
            "&"
        } else {
            "?"
        }
        return "$baseUrl${separator}token=$token"
    }

    private companion object {
        private const val HUB_SESSION_ID = "hub-overview"
        private const val MESSAGE_BUFFER_LIMIT = 500
        private const val RPC_TIMEOUT_MS = 30_000L
    }
}
