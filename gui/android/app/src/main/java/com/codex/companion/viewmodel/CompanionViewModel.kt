package com.codex.companion.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.codex.companion.data.repository.CompanionRepository
import com.codex.companion.model.CompanionRole
import com.codex.companion.model.ThreadControlAction
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class CompanionViewModel(
    private val repository: CompanionRepository,
) : ViewModel() {
    private val mutableUiState = MutableStateFlow(CompanionUiState())
    val uiState: StateFlow<CompanionUiState> = mutableUiState.asStateFlow()

    init {
        viewModelScope.launch {
            val latest = repository.loadLatestPairing()
            if (latest != null) {
                mutableUiState.update {
                    it.copy(
                        host = latest.host,
                        token = latest.token,
                        code = latest.pairingCode,
                        isPaired = true,
                        pairedHost = latest.host,
                        statusMessage = "Loaded previous pairing.",
                        role = latest.role,
                    )
                }
            }
        }

        viewModelScope.launch {
            repository.connectionState.collect { state ->
                mutableUiState.update { it.copy(connectionState = state) }
            }
        }

        viewModelScope.launch {
            repository.role.collect { role ->
                mutableUiState.update { it.copy(role = role) }
            }
        }

        viewModelScope.launch {
            repository.sessions().collect { sessions ->
                mutableUiState.update { it.copy(sessions = sessions) }
            }
        }
    }

    fun onHostChanged(host: String) {
        mutableUiState.update { it.copy(host = host) }
    }

    fun onTokenChanged(token: String) {
        mutableUiState.update { it.copy(token = token) }
    }

    fun onCodeChanged(code: String) {
        mutableUiState.update { it.copy(code = code) }
    }

    fun onPromptChanged(prompt: String) {
        mutableUiState.update { it.copy(promptDraft = prompt) }
    }

    fun onPairRequested() {
        val snapshot = mutableUiState.value
        if (snapshot.host.isBlank() || snapshot.token.isBlank() || snapshot.code.isBlank()) {
            mutableUiState.update { it.copy(statusMessage = "Host, token, and code are required.") }
            return
        }

        viewModelScope.launch {
            repository.pairDevice(
                host = snapshot.host,
                token = snapshot.token,
                code = snapshot.code,
                role = snapshot.role,
            )
            mutableUiState.update {
                it.copy(
                    isPaired = true,
                    pairedHost = snapshot.host,
                    statusMessage = "Pairing request sent. Waiting for host connection.",
                )
            }
        }
    }

    fun onSessionOpened(sessionId: String) {
        viewModelScope.launch {
            repository.threadMessages(sessionId).collectLatest { messages ->
                mutableUiState.update { state -> state.copy(currentThreadMessages = messages) }
            }
        }
    }

    fun onSendPrompt(sessionId: String) {
        val prompt = mutableUiState.value.promptDraft
        if (prompt.isBlank()) {
            mutableUiState.update { it.copy(statusMessage = "Prompt is empty.") }
            return
        }

        viewModelScope.launch {
            repository.sendPrompt(sessionId, prompt)
            repository.saveSessionCursor(sessionId, cursor = "last-local-send")
            mutableUiState.update {
                it.copy(
                    promptDraft = "",
                    statusMessage = "Prompt queued for delivery.",
                )
            }
        }
    }

    fun onActionTapped(sessionId: String, action: ThreadControlAction) {
        viewModelScope.launch {
            repository.sendAction(sessionId, action)
            mutableUiState.update { it.copy(statusMessage = "$action requested for $sessionId.") }
        }
    }

    fun onRoleChanged(role: CompanionRole) {
        mutableUiState.update { it.copy(role = role, statusMessage = "Role set to ${role.name}.") }
    }
}
