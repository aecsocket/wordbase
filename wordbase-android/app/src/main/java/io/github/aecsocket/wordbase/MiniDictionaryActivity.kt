package io.github.aecsocket.wordbase

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import com.kevinnzou.web.WebView
import com.kevinnzou.web.rememberWebViewStateWithHTMLData
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import uniffi.wordbase_api.RecordKind

class MiniDictionaryActivity : ComponentActivity() {
    @OptIn(ExperimentalMaterial3Api::class)
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val query = intent.getStringExtra(Intent.EXTRA_PROCESS_TEXT) ?: run {
            finish()
            return
        }

        enableEdgeToEdge()
        setContent {
            WordbaseTheme {
                var html by remember { mutableStateOf("") }

                LaunchedEffect(Unit) {
                    val wordbase = wordbase()

                    val records = wordbase.lookup(
                        profileId = 1L,
                        sentence = query,
                        cursor = 0UL,
                        recordKinds = RecordKind.entries
                    )
                    html = wordbase.renderToHtml(
                        records = records,
                        accentColorR = 0x35U,
                        accentColorG = 0x84U,
                        accentColorB = 0xe4U,
                    ) + "<style>body { margin: 0 0 16px 0; }</style>"
                }

                ModalBottomSheet(
                    onDismissRequest = {
                        finish()
                    }, modifier = Modifier.fillMaxSize()
                ) {
                    // the column lets you scroll the webview
                    // why? idk!!
                    // https://medium.com/@itsuki.enjoy/android-kotlin-jetpack-compose-make-your-nested-webview-scroll-cbf023e821a1
                    LazyColumn {
                        item {
                            val webViewState = rememberWebViewStateWithHTMLData(html)
                            WebView(
                                state = webViewState,
                                modifier = Modifier.fillMaxSize(),
                            )
                        }
                    }
                }
            }
        }
    }
}
