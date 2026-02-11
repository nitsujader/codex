package com.codex.companion.data.discovery

import kotlinx.coroutines.flow.Flow

interface MdnsDiscovery {
    fun discoverPeers(): Flow<List<DiscoveredPeer>>
}

data class DiscoveredPeer(
    val serviceName: String,
    val host: String,
    val port: Int,
)
