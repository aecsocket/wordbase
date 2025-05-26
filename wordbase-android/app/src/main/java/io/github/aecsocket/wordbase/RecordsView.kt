package io.github.aecsocket.wordbase

import android.annotation.SuppressLint
import android.util.Log
import android.webkit.JavascriptInterface
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.contentColorFor
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.ichi2.anki.api.AddContentApi
import com.kevinnzou.web.WebView
import com.kevinnzou.web.rememberWebViewNavigator
import com.kevinnzou.web.rememberWebViewStateWithHTMLData
import kotlinx.coroutines.launch
import uniffi.wordbase.RenderConfig
import uniffi.wordbase.Wordbase
import uniffi.wordbase.WordbaseException
import uniffi.wordbase_api.RecordEntry
import uniffi.wordbase_api.RecordKind
import uniffi.wordbase_api.Term

const val TAG = "RecordsView"

@Composable
fun rememberLookup(
    wordbase: Wordbase,
    sentence: String,
    cursor: ULong,
    onRecords: (List<RecordEntry>) -> Unit = {},
): List<RecordEntry> {
    var records by remember { mutableStateOf(listOf<RecordEntry>()) }

    val app = LocalContext.current.app()
    LaunchedEffect(arrayOf(sentence, cursor, app.dictionaries, app.profiles, app.profileId)) {
        records = wordbase.lookup(
            profileId = app.profileId,
            sentence = sentence,
            cursor = cursor,
            recordKinds = RecordKind.entries,
        )
        onRecords(records)
    }

    return records
}

@Composable
fun RecordsView(
    wordbase: Wordbase,
    snackbarHostState: SnackbarHostState,
    sentence: String,
    cursor: ULong,
    records: List<RecordEntry> = rememberLookup(wordbase, sentence, cursor),
    insets: WindowInsets = WindowInsets(0.dp),
    containerColor: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = contentColorFor(containerColor),
    onExit: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    val app = context.app()
    val coroutineScope = rememberCoroutineScope()

    suspend fun addNote(term: Term) {
        Log.i(TAG, "Adding card for (${term.headword}, ${term.reading})")
        if (AddContentApi.getAnkiDroidPackageName(context) == null) {
            snackbarHostState.showSnackbar(
                message = "Anki is not installed"
            )
            return
        }
        val anki = AddContentApi(context)

        val deckName = "Testing"
        val modelName = "Lapis"

        val deckId = anki.deckList
            .firstNotNullOfOrNull { (id, name) -> if (name == deckName) id else null }
            ?: run {
                snackbarHostState.showSnackbar(
                    message = "No deck named '$deckName'"
                )
                return
            }
        val modelId = anki.modelList
            .firstNotNullOfOrNull { (id, name) -> if (name == modelName) id else null }
            ?: run {
                snackbarHostState.showSnackbar(
                    message = "No note type named '$modelName'"
                )
                return
            }

        val fieldNames = anki.getFieldList(modelId) ?: run {
            snackbarHostState.showSnackbar(
                message = "Failed to get fields for '$modelName'"
            )
            return
        }

        val termNote = try {
            wordbase.buildTermNote(
                profileId = app.profileId,
                sentence = sentence,
                cursor = cursor,
                term = term,
            )
        } catch (ex: WordbaseException) {
            snackbarHostState.showSnackbar(
                message = "Failed to build term note: $ex"
            )
            return
        }

        val fields = fieldNames.map { fieldName ->
            termNote.fields[fieldName] ?: ""
        }

        anki.addNote(modelId, deckId, fields.toTypedArray(), setOf("wordbase"))
            ?: run {
                snackbarHostState.showSnackbar(
                    message = "Failed to add note"
                )
            }
    }

    RawRecordsView(
        wordbase = wordbase,
        records = records,
        insets = insets,
        containerColor = containerColor,
        contentColor = contentColor,
        onAddNote = { term ->
            coroutineScope.launch {
                addNote(term)
            }
        },
        onExit = onExit,
    )
}

@Composable
fun RawRecordsView(
    wordbase: Wordbase,
    records: List<RecordEntry>,
    insets: WindowInsets = WindowInsets(0.dp),
    containerColor: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = contentColorFor(containerColor),
    onAddNote: ((Term) -> Unit)? = null,
    onExit: (() -> Unit)? = null,
) {
    class JsBridge(val onAddNote: (Term) -> Unit) {
        @Suppress("unused") // used by JS
        @JavascriptInterface
        fun addCard(headword: String?, reading: String?) {
            onAddNote(Term(headword = headword, reading = reading))
        }
    }

    // amazingly, this scales perfectly
    val density = LocalDensity.current
    val layoutDir = LocalLayoutDirection.current
    val extraCss = with(density) {
        """
        :root {
            --bg-color: ${containerColor.css()};
            --fg-color: ${contentColor.css()};
            --accent-color: ${MaterialTheme.colorScheme.primary.css()};
            --on-accent-color: ${MaterialTheme.colorScheme.onPrimary.css()};
        }

        .content {
            margin:
                0
                ${insets.getRight(density, layoutDir).toDp().value}px
                0
                ${insets.getLeft(density, layoutDir).toDp().value}px;
        }

        body {
            padding:
                ${insets.getTop(density).toDp().value}px
                0
                ${insets.getBottom(density).toDp().value}px
                0;
        }
        """.trimIndent()
    }
    val html = wordbase.renderToHtml(
        records = records,
        config = RenderConfig(
            addNoteText = stringResource(R.string.add_note),
            addNoteJsFn = null,
//            addNoteJsFn = "Wordbase.addNote",
        ),
    ) + "<style>$extraCss</style>"

    val webViewState = rememberWebViewStateWithHTMLData(html)
    val navigator = rememberWebViewNavigator()
    WebView(
        state = webViewState,
        navigator = navigator,
        modifier = Modifier
            .fillMaxSize()
            // Sometimes during recomposition (and loading new HTML),
            // the WebView will briefly flash to a default state.
            // During this, it will flash to its background color.
            // But for some reason, it will also draw OVER some other content.
            // (E.g. the search bar in the main activity view).
            // To prevent this, we clip the WebView like so.
            .graphicsLayer { clip = true },
        captureBackPresses = false,
        onCreated = {
            it.setBackgroundColor(containerColor.toArgb())
            it.settings.allowFileAccess = false
            it.settings.allowContentAccess = false

            @SuppressLint("SetJavaScriptEnabled") // it is what it is
            it.settings.javaScriptEnabled = true
            onAddNote?.let { onAddCard ->
                @SuppressLint("JavascriptInterface") // false positive
                it.addJavascriptInterface(JsBridge(onAddCard), "Wordbase")
            }
        }
    )

    BackHandler {
        if (navigator.canGoBack) {
            navigator.navigateBack()
        } else {
            onExit?.invoke()
        }
    }
}

@Composable
fun NoRecordsView() {
    StatusPage {
        StatusPageTitle(
            text = stringResource(R.string.records_empty),
        )
    }
}

@Composable
fun StatusPage(content: @Composable BoxScope.() -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(
                horizontal = 64.dp,
                vertical = 96.dp,
            ),
        contentAlignment = Alignment.Center,
        content = content,
    )
}

@Composable
fun StatusPageTitle(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.headlineLarge,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
    )
}

fun Color.css() = "rgb(${red * 100}% ${green * 100}% ${blue * 100}%)"
