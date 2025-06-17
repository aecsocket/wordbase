package io.github.aecsocket.wordbase

import android.content.Intent
import android.util.Log
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.ichi2.anki.api.AddContentApi
import com.multiplatform.webview.jsbridge.IJsMessageHandler
import com.multiplatform.webview.jsbridge.JsMessage
import com.multiplatform.webview.jsbridge.WebViewJsBridge
import com.multiplatform.webview.web.WebView
import com.multiplatform.webview.web.WebViewNavigator
import com.multiplatform.webview.web.rememberWebViewNavigator
import com.multiplatform.webview.web.rememberWebViewStateWithHTMLData
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toPersistentList
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.json.JSONObject
import uniffi.wordbase.NoteField
import uniffi.wordbase.RenderConfig
import uniffi.wordbase.Wordbase
import uniffi.wordbase.WordbaseException
import uniffi.wordbase_api.RecordEntry
import uniffi.wordbase_api.Term

private const val TAG = "RecordsView"

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
        ).toPersistentList()
        onEntries(records)
    }

    return records
}

@Composable
fun noteExistsFn(deck: String, noteType: String): suspend (Term) -> Boolean {
    val context = LocalContext.current

    return fn@{ term ->
        val ankiDroidPackage = AddContentApi.getAnkiDroidPackageName(context)
        if (ankiDroidPackage == null) {
            return@fn false
        }
        val anki = AddContentApi(context)

        val noteTypeId = anki.modelList
            .firstNotNullOfOrNull { (id, name) -> if (name == noteType) id else null }
            ?: return@fn false

        // TODO: note key is term.headword
        anki.findDuplicateNotes(noteTypeId, term.headword).isNotEmpty()
    }
}

@Composable
fun addNoteFn(
    wordbase: Wordbase,
    sentence: String,
    entries: ImmutableList<RecordEntry>,
    deck: String,
    noteType: String,
): suspend (Term) -> Unit {
    val context = LocalContext.current

    fun toast(text: String) {
        CoroutineScope(Dispatchers.Main).launch {
            Toast.makeText(context, text, Toast.LENGTH_SHORT).show()
        }
    }

    val textNoAnki = stringResource(R.string.add_note_no_anki)
    val textNoDeck = stringResource(R.string.add_note_no_deck)
    val textNoNoteType = stringResource(R.string.add_note_no_note_type)
    val textErrGetFields = stringResource(R.string.add_note_err_get_fields)
    val textErrBuildNote = stringResource(R.string.add_note_err_build_note)
    val textErrDuplicate = stringResource(R.string.add_note_err_duplicate)
    val textErrAdd = stringResource(R.string.add_note_err_add)
    val textAdded = stringResource(R.string.add_note_added)

    return fn@{ term ->
        Log.i(
            TAG,
            """
                Adding card for ${term.asString()}
                  "$sentence"
                  Terms: ${entries.map { it.term }.toSet()}
            """.trimIndent()
        )

        val ankiDroidPackage = AddContentApi.getAnkiDroidPackageName(context)
        if (ankiDroidPackage == null) {
            toast(textNoAnki)
            return@fn
        }
        val anki = AddContentApi(context)

        val deckId = anki.deckList
            .firstNotNullOfOrNull { (id, name) -> if (name == deck) id else null }
            ?: run {
                toast(textNoDeck.format(deck))
                return@fn
            }
        val noteTypeId = anki.modelList
            .firstNotNullOfOrNull { (id, name) -> if (name == noteType) id else null }
            ?: run {
                toast(textNoNoteType.format(noteType))
                return@fn
            }

        val fieldNames = anki.getFieldList(noteTypeId) ?: run {
            toast(textErrGetFields.format(noteType))
            return@fn
        }

        val termNote = try {
            wordbase.buildTermNote(
                entries = entries,
                sentence = sentence,
                term = term,
            )
        } catch (ex: WordbaseException) {
            toast(textErrBuildNote)
            ex.printStackTrace()
            return@fn
        }

        // TODO: note key is term.headword
        if (anki.findDuplicateNotes(noteTypeId, term.headword).isNotEmpty()) {
            toast(textErrDuplicate)
            return@fn
        }

        val fields = fieldNames.map { fieldName ->
            when (val field = termNote.fields[fieldName]) {
                is NoteField.String -> field.v1
                is NoteField.Audio -> {
                    context.grantUriPermission(ankiDroidPackage, NoteProvider.uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                    NoteProvider.data = field.v1

                    val fieldContent = anki.addMediaFromUri(
                        NoteProvider.uri,
                        "wordbase",
                        "audio"
                    )

                    context.revokeUriPermission(NoteProvider.uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                    NoteProvider.data = null
                    fieldContent ?: error("Failed to add media to AnkiDroid")
                }
                null -> ""
            }
        }

        val noteId = anki.addNote(noteTypeId, deckId, fields.toTypedArray(), setOf("wordbase"))
        if (noteId == null) {
            toast(textErrAdd)
        } else {
            toast(textAdded.format(term.displayString()))
        }
    }
}

@Composable
fun viewNoteFn(): suspend (Term) -> Unit {
    return fn@{ term -> }
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

    val deck by derivedStateOf { app.profile?.ankiDeck }
    val noteType by derivedStateOf { app.profile?.ankiNoteType }

    var onNoteExists: (suspend (Term) -> Boolean)? = null
    var onAddNote: (suspend (Term) -> Unit)? = null
    var onViewNote: (suspend (Term) -> Unit)? = null

    deck?.let { deck ->
        noteType?.let { noteType ->
            onNoteExists = noteExistsFn(deck = deck, noteType = noteType)
            onAddNote = addNoteFn(
                wordbase = wordbase,
                sentence = sentence,
                entries = entries,
                deck = deck,
                noteType = noteType,
            )
            onViewNote = viewNoteFn()
        }
    }

    RawRecordsView(
        wordbase = wordbase,
        entries = entries,
        insets = insets,
        containerColor = containerColor,
        contentColor = contentColor,
        onNoteExists = onNoteExists,
        onAddNote = onAddNote,
        onViewNote = onViewNote,
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
    onNoteExists: (suspend (Term) -> Boolean)? = null,
    onAddNote: (suspend (Term) -> Unit)? = null,
    onViewNote: (suspend (Term) -> Unit)? = null,
    onExit: (() -> Unit)? = null,
) {
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
    val sViewNote = stringResource(R.string.view_note)
    val sAddDuplicateNote = stringResource(R.string.add_duplicate_note)
    val document by derivedStateOf {
        fun jsCall(method: String, callback: Boolean = false) = """
            wait_for_wordbase(() => window.wordbase.callNative(
                '$method',
                JSON.stringify({
                    headword: <js_headword>,
                    reading: <js_reading>,
                }),
                ${if (callback) "json => <js_callback>(JSON.parse(json))" else "null"},
            ))
        """.trimIndent()

        val body = wordbase.renderHtmlBody(
            entries = entries,
            config = RenderConfig(
                sAddNote = sAddNote,
                sViewNote = sViewNote,
                sAddDuplicateNote = sAddDuplicateNote,
                fnNoteExists = jsCall("note_exists", callback = true),
                fnAddNote = jsCall("add_note"),
                fnViewNote = jsCall("view_note"),
            ),
        )

        // https://github.com/KevinnZou/compose-webview-multiplatform/issues/238
        """
        <!doctype html>
        <html>
            <body>
                <script>
                    function wait_for_wordbase(callback) {
                        if (window.wordbase !== undefined) {
                            callback();
                        } else {
                            setTimeout(() => wait_for_wordbase(callback), 50);
                        }
                    }
                </script>

                $body
                <style>$extraCss</style>
            </body>
        </html>
        """.trimIndent()
    }

    val webViewState = rememberWebViewStateWithHTMLData(document)
    var webViewInitialized by remember { mutableStateOf(false) }

    LaunchedEffect(entries, webViewInitialized) {
        if (webViewInitialized) {
            webViewState.nativeWebView.clearHistory()
        }
    }

    val navigator = rememberWebViewNavigator()
    BackHandler {
        if (navigator.canGoBack) {
            navigator.navigateBack()
        } else {
            onExit?.invoke()
        }
    }

    val jsBridge = remember {
        WebViewJsBridge(
            navigator = navigator,
            jsBridgeName = "wordbase",
        )
    }

    @Composable
    fun <R> registerJsFunction(methodName: String, fn: (suspend (Term) -> R?)?) {
        var jsHandler by remember { mutableStateOf<IJsMessageHandler?>(null) }
        LaunchedEffect(jsBridge, fn) {
            println("UnRegistered $methodName")
            jsHandler?.let { jsBridge.unregister(it) }
            fn?.let { fn ->
                println("Registered $methodName")
                val handler = object : IJsMessageHandler {
                    override fun methodName() = methodName

                    override fun handle(
                        message: JsMessage,
                        navigator: WebViewNavigator?,
                        callback: (String) -> Unit
                    ) {
                        val json = JSONObject(message.params)
                        // spent like 2 hours trying to figure out why `reading` was "null", not null
                        // a proper type system fixes this!!! fucking java legacy code!!!
                        val headword = if (json.isNull(HEADWORD)) null else json.getString("headword")
                        val reading = if (json.isNull(READING)) null else json.getString("reading")
                        val term = Term(
                            headword = headword,
                            reading = reading,
                        )

                        CoroutineScope(Dispatchers.IO).launch {
                            fn(term)?.let { r -> callback(r.toString()) }
                        }
                    }
                }

                jsBridge.register(handler)
                jsHandler = handler
            }
        }
    }

    registerJsFunction("note_exists", onNoteExists)
    registerJsFunction("add_note", onAddNote)
    registerJsFunction("view_note", onViewNote)

//    jsBridge.register(object : IJsMessageHandler {
//        override fun methodName() = "hello_world"
//
//        override fun handle(
//            message: JsMessage,
//            navigator: WebViewNavigator?,
//            callback: (String) -> Unit
//        ) {
//
//        }
//    })

    WebView(
        state = webViewState,
        navigator = navigator,
        webViewJsBridge = jsBridge,
        modifier = Modifier.fillMaxSize(),
        captureBackPresses = false,
        onCreated = { webView ->
            webView.setBackgroundColor(containerColor.toArgb())
            webView.settings.allowFileAccess = false
            webView.settings.allowContentAccess = false
            webViewInitialized = true
        }
    )
}

const val HEADWORD = "headword"
const val READING = "reading"

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

fun Term.asString(): String {
    val headword = headword?.let { "\"$headword\"" } ?: "-"
    val reading = reading?.let { "\"$reading\"" } ?: "-"
    return "($headword, $reading)"
}

fun Term.displayString() = headword?.let { headword ->
    reading?.let { reading ->
        "$headword ($reading)"
    } ?: headword
} ?: reading ?: "?"
