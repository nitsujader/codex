package com.codex.companion.ui

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.codex.companion.model.ThreadControlAction
import com.codex.companion.ui.screens.LiveThreadScreen
import com.codex.companion.ui.screens.PairingScreen
import com.codex.companion.ui.screens.SessionListScreen
import com.codex.companion.ui.screens.SettingsScreen
import com.codex.companion.viewmodel.CompanionViewModel

private object Routes {
    const val Pairing = "pairing"
    const val Sessions = "sessions"
    const val Settings = "settings"
    const val Thread = "thread"
}

@Composable
fun CompanionApp(
    viewModel: CompanionViewModel,
    modifier: Modifier = Modifier,
) {
    val navController = rememberNavController()
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val backStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = backStackEntry?.destination?.route

    LaunchedEffect(uiState.isPaired, currentRoute) {
        if (uiState.isPaired && currentRoute == Routes.Pairing) {
            navController.navigate(Routes.Sessions) {
                popUpTo(Routes.Pairing) {
                    inclusive = true
                }
            }
        }
    }

    Scaffold(
        modifier = modifier,
        bottomBar = {
            if (uiState.isPaired) {
                NavigationBar {
                    NavigationBarItem(
                        selected = currentRoute == Routes.Sessions,
                        onClick = {
                            navController.navigate(Routes.Sessions) {
                                popUpTo(navController.graph.findStartDestination().id) {
                                    saveState = true
                                }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                        icon = {
                            Icon(
                                imageVector = Icons.Default.List,
                                contentDescription = "Sessions",
                            )
                        },
                        label = { Text("Sessions") },
                    )
                    NavigationBarItem(
                        selected = currentRoute == Routes.Settings,
                        onClick = {
                            navController.navigate(Routes.Settings) {
                                popUpTo(navController.graph.findStartDestination().id) {
                                    saveState = true
                                }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                        icon = {
                            Icon(
                                imageVector = Icons.Default.Settings,
                                contentDescription = "Settings",
                            )
                        },
                        label = { Text("Settings") },
                    )
                }
            }
        },
    ) { innerPadding ->
        NavHost(
            navController = navController,
            startDestination = Routes.Pairing,
            modifier = Modifier,
        ) {
            composable(Routes.Pairing) {
                PairingScreen(
                    uiState = uiState,
                    innerPadding = innerPadding,
                    onHostChanged = viewModel::onHostChanged,
                    onTokenChanged = viewModel::onTokenChanged,
                    onCodeChanged = viewModel::onCodeChanged,
                    onPairRequested = viewModel::onPairRequested,
                )
            }
            composable(Routes.Sessions) {
                SessionListScreen(
                    sessions = uiState.sessions,
                    innerPadding = innerPadding,
                    onSessionSelected = { sessionId ->
                        viewModel.onSessionOpened(sessionId)
                        navController.navigate("${Routes.Thread}/$sessionId")
                    },
                )
            }
            composable(
                route = "${Routes.Thread}/{sessionId}",
                arguments = listOf(navArgument("sessionId") { type = NavType.StringType }),
            ) { entry ->
                val sessionId = entry.arguments?.getString("sessionId") ?: return@composable
                LiveThreadScreen(
                    sessionId = sessionId,
                    messages = uiState.currentThreadMessages,
                    promptDraft = uiState.promptDraft,
                    innerPadding = innerPadding,
                    onPromptChanged = viewModel::onPromptChanged,
                    onSendPrompt = { viewModel.onSendPrompt(sessionId) },
                    onInterrupt = { viewModel.onActionTapped(sessionId, ThreadControlAction.INTERRUPT) },
                    onPause = { viewModel.onActionTapped(sessionId, ThreadControlAction.PAUSE) },
                    onResume = { viewModel.onActionTapped(sessionId, ThreadControlAction.RESUME) },
                )
            }
            composable(Routes.Settings) {
                SettingsScreen(
                    connectionState = uiState.connectionState,
                    role = uiState.role,
                    pairedHost = uiState.pairedHost,
                    innerPadding = innerPadding,
                    onRoleChanged = viewModel::onRoleChanged,
                )
            }
        }
    }
}
