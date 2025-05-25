package io.github.aecsocket.wordbase

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.BottomSheetDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.core.view.HapticFeedbackConstantsCompat
import androidx.core.view.ViewCompat
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import kotlinx.coroutines.launch

class MiniDictionaryActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val sentence = intent.getStringExtra(Intent.EXTRA_PROCESS_TEXT) ?: run {
            finish()
            return
        }

        enableEdgeToEdge()
        setContent {
            WordbaseTheme {
                MiniDictionaryUi(sentence = sentence)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MiniDictionaryUi(sentence: String) {
    data class Cursor(
        val chars: Int = 0,
        val bytes: ULong = 0UL,
    )

    var wordbase by rememberWordbase()
    var cursor by remember { mutableStateOf(Cursor()) }
    var scanChars by remember { mutableStateOf(0UL) }

    val view = LocalView.current
    val annotatedQuery = buildAnnotatedString {
        var charIndex = 0
        var byteIndex = 0UL
        for (ch in sentence) {
            val thisCharIndex = charIndex
            val thisByteIndex = byteIndex

            append(ch)
            addLink(
                clickable = LinkAnnotation.Clickable(
                    tag = "",
                    linkInteractionListener = {
                        if (cursor.chars == thisCharIndex) {
                            // don't trigger recomposition/re-lookup
                            return@Clickable
                        }
                        cursor = Cursor(
                            chars = thisCharIndex,
                            bytes = thisByteIndex,
                        )
                        scanChars = 1UL

                        ViewCompat.performHapticFeedback(
                            view,
                            HapticFeedbackConstantsCompat.CONTEXT_CLICK,
                        )
                    }
                ),
                start = thisCharIndex,
                end = thisCharIndex + 1,
            )

            charIndex += 1
            byteIndex += ch.toString().toByteArray(Charsets.UTF_8).size.toUInt()
        }

        addStyle(
            style = SpanStyle(
                color = MaterialTheme.colorScheme.surface,
                background = MaterialTheme.colorScheme.primary,
            ),
            start = cursor.chars,
            end = cursor.chars + scanChars.toInt(),
        )
    }

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
            item {
                Box(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                ) {
                    SelectionContainer {
                        Text(
                            text = annotatedQuery,
                            style = MaterialTheme.typography.headlineSmall,
                            color = MaterialTheme.colorScheme.primary,
                            softWrap = false,
                            maxLines = 1,
                            modifier = Modifier.padding(
                                horizontal = 16.dp,
                                vertical = 4.dp,
                            )
                        )
                    }
                }
            }

            item {
                wordbase?.let { wordbase ->
                    val records = rememberRecordLookup(
                        wordbase = wordbase,
                        profileId = 1L,
                        sentence = sentence,
                        cursor = cursor.bytes,
                    )
                    scanChars = records.maxOfOrNull { it.charsScanned } ?: 1UL

                    if (records.isEmpty()) {
                        NoRecordsView()
                    } else {
                        RecordsView(
                            wordbase = wordbase,
                            records = records,
                            containerColor = BottomSheetDefaults.ContainerColor,
                            onExit = {
                                coroutineScope.launch {
                                    sheetState.hide()
                                    activity?.finish()
                                }
                            },
                        )
                    }
                } ?: run {
                    Box(
                        modifier = Modifier.fillMaxWidth(),
                        contentAlignment = Alignment.Center,
                    ) {
                        CircularProgressIndicator(
                            modifier = Modifier
                                .height(128.dp)
                                .aspectRatio(1f)
                                .padding(16.dp)
                        )
                    }
                }
            }
        }
    }
}
