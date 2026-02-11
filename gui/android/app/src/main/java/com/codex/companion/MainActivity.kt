package com.codex.companion

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.lifecycle.viewmodel.compose.viewModel
import com.codex.companion.ui.CompanionApp
import com.codex.companion.viewmodel.CompanionViewModel
import com.codex.companion.viewmodel.CompanionViewModelFactory

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val appContainer = (application as CompanionApplication).appContainer

        setContent {
            MaterialTheme {
                Surface {
                    val viewModel: CompanionViewModel = viewModel(
                        factory = CompanionViewModelFactory(appContainer.repository),
                    )
                    CompanionApp(viewModel = viewModel)
                }
            }
        }
    }
}
