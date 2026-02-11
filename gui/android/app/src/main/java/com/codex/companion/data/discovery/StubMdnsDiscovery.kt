package com.codex.companion.data.discovery

import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow

class StubMdnsDiscovery : MdnsDiscovery {
    override fun discoverPeers(): Flow<List<DiscoveredPeer>> = flow {
        // TODO: Replace with Android NSD/mDNS service discovery implementation.
        emit(emptyList())
        while (true) {
            delay(30_000)
            emit(emptyList())
        }
    }
}
