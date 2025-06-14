package io.github.aecsocket.wordbase

import android.annotation.SuppressLint
import android.util.Log
import android.webkit.JavascriptInterface
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.contentColorFor
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
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
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toPersistentList
import kotlinx.coroutines.launch
import uniffi.wordbase.RenderConfig
import uniffi.wordbase.Wordbase
import uniffi.wordbase.WordbaseException
import uniffi.wordbase_api.RecordEntry
import uniffi.wordbase_api.RecordId
import uniffi.wordbase_api.RecordKind
import uniffi.wordbase_api.Term

const val TAG = "RecordsView"

@Composable
fun rememberLookup(
    wordbase: Wordbase,
    sentence: String,
    cursor: ULong,
    onEntries: (ImmutableList<RecordEntry>) -> Unit = {},
): ImmutableList<RecordEntry> {
    var records by remember { mutableStateOf(persistentListOf<RecordEntry>()) }

    val app = LocalContext.current.app()
    LaunchedEffect(arrayOf(sentence, cursor, app.dictionaries, app.profiles, app.profileId)) {
        records = wordbase.lookup(
            profileId = app.profileId,
            sentence = sentence,
            cursor = cursor,
            recordKinds = RecordKind.entries,
        ).toPersistentList()
        onEntries(records)
    }

    return records
}

@Composable
fun RecordsView(
    wordbase: Wordbase,
    sentence: String,
    cursor: ULong,
    entries: ImmutableList<RecordEntry> = rememberLookup(wordbase, sentence, cursor),
    insets: WindowInsets = WindowInsets(0.dp),
    containerColor: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = contentColorFor(containerColor),
    onExit: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    val app = context.app()
    val coroutineScope = rememberCoroutineScope()

    val textNoAnki = stringResource(R.string.add_note_no_anki)
    val textNoDeck = stringResource(R.string.add_note_no_deck)
    val textNoNoteType = stringResource(R.string.add_note_no_note_type)
    val textErrGetFields = stringResource(R.string.add_note_err_get_fields)
    val textErrBuildNote = stringResource(R.string.add_note_err_build_note)
    val textErrAdd = stringResource(R.string.add_note_err_add)
    val textAdded = stringResource(R.string.add_note_added)

    suspend fun addNote(term: Term, deckName: String, modelName: String) {
        Log.i(TAG, "Adding card for (${term.headword}, ${term.reading})")
        if (AddContentApi.getAnkiDroidPackageName(context) == null) {
            Toast.makeText(context, textNoAnki, Toast.LENGTH_SHORT).show()
            return
        }
        val anki = AddContentApi(context)

        val deckId = anki.deckList
            .firstNotNullOfOrNull { (id, name) -> if (name == deckName) id else null }
            ?: run {
                Toast.makeText(context, textNoDeck.format(deckName), Toast.LENGTH_SHORT).show()
                return
            }
        val modelId = anki.modelList
            .firstNotNullOfOrNull { (id, name) -> if (name == modelName) id else null }
            ?: run {
                Toast.makeText(context, textNoNoteType.format(modelName), Toast.LENGTH_SHORT).show()
                return
            }

        val fieldNames = anki.getFieldList(modelId) ?: run {
            Toast.makeText(context, textErrGetFields.format(modelName), Toast.LENGTH_SHORT)
                .show()
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
            Toast.makeText(context, textErrBuildNote, Toast.LENGTH_SHORT).show()
            ex.printStackTrace()
            return
        }

        val fields = fieldNames.map { fieldName ->
            termNote.fields[fieldName] ?: ""
        }

        val noteId = anki.addNote(modelId, deckId, fields.toTypedArray(), setOf("wordbase"))
        if (noteId == null) {
            Toast.makeText(context, textErrAdd, Toast.LENGTH_SHORT).show()
        } else {
            Toast.makeText(context, textAdded, Toast.LENGTH_SHORT).show()
        }
    }

    val onAddNote = app.profile?.ankiDeck?.let { ankiDeck ->
        app.profile?.ankiNoteType?.let { noteType ->
            { term: Term ->
                coroutineScope.launch {
                    addNote(term, ankiDeck, noteType)
                }
                Unit
            }
        }
    }

    RawRecordsView(
        wordbase = wordbase,
        entries = entries,
        insets = insets,
        containerColor = containerColor,
        contentColor = contentColor,
        onAddNote = onAddNote,
        onExit = onExit,
    )
}

@Composable
fun RawRecordsView(
    wordbase: Wordbase,
    entries: ImmutableList<RecordEntry>,
    insets: WindowInsets = WindowInsets(0.dp),
    containerColor: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = contentColorFor(containerColor),
    onAddNote: ((Term) -> Unit)? = null,
    onExit: (() -> Unit)? = null,
) {
    @Suppress("unused") // used by JS
    class JsBridge(val onAddNote: (Term) -> Unit) {
        @JavascriptInterface
        fun addNote(headword: String?, reading: String?) {
            onAddNote(Term(headword = headword, reading = reading))
        }

        @JavascriptInterface
        fun audioBlob(recordId: RecordId): String {
            return "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEAwFIAfgAAABAAEAA8AQAAEpgADhEAARUAIgcEAgAAh0EoHy1qAAAOoBwaqABhUHQAAAA=";
        }
    }

    // amazingly, this scales perfectly
    val density = LocalDensity.current
    val layoutDir = LocalLayoutDirection.current
    val colorScheme = MaterialTheme.colorScheme
    val extraCss by derivedStateOf {
        with(density) {
            """
            :root {
                --bg-color: ${containerColor.css()};
                --fg-color: ${contentColor.css()};
                --accent-color: ${colorScheme.primary.css()};
                --on-accent-color: ${colorScheme.onPrimary.css()};
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
    }


    val sAddNote = stringResource(R.string.add_note)
    val document by derivedStateOf {
        val html = wordbase.renderHtml(
            entries = entries,
            config = RenderConfig(
                sAddNote = sAddNote,
                fnAddNote = "WordbaseAndroid.addNote",
                fnAudioBlob = "WordbaseAndroid.audioBlob",
            ),
        )

        println("rebuilt document!")

        """
        <!doctype html>
        <html>
            <body>
                ${html.body}
                <style>$extraCss</style>
            </body>
        </html>
        """.trimIndent()
    }

    val webViewState = rememberWebViewStateWithHTMLData(document)
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
            onAddNote?.let { onAddNote ->
                @SuppressLint("JavascriptInterface") // false positive
                it.addJavascriptInterface(JsBridge(onAddNote), "WordbaseAndroid")
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
