package io.github.aecsocket.wordbase

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.BottomSheetDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import kotlinx.coroutines.launch

class MiniDictionaryActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val query = intent.getStringExtra(Intent.EXTRA_PROCESS_TEXT) ?: run {
            finish()
            return
        }

        enableEdgeToEdge()
        setContent {
            WordbaseTheme {
                Ui(query = query)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Ui(query: String) {
    var wordbase by rememberWordbase()

    val coroutineScope = rememberCoroutineScope()
    val sheetState = rememberModalBottomSheetState()
    val activity = LocalActivity.current
    ModalBottomSheet(
        sheetState = sheetState,
        onDismissRequest = {
            activity?.finish()
        },
        modifier = Modifier
            .fillMaxWidth()
            .statusBarsPadding()
    ) {
        // the column lets you scroll the webview
        // why? idk!!
        // https://medium.com/@itsuki.enjoy/android-kotlin-jetpack-compose-make-your-nested-webview-scroll-cbf023e821a1
        LazyColumn {
            wordbase?.let { wordbase ->
                item {
                    LookupView(
                        wordbase = wordbase,
                        query = query,
                        padding = PaddingValues(0.dp),
                        containerColor = BottomSheetDefaults.ContainerColor,
                        onExit = {
                            coroutineScope.launch {
                                sheetState.hide()
                                activity?.finish()
                            }
                        }
                    )
                }
            }
        }
    }
}
