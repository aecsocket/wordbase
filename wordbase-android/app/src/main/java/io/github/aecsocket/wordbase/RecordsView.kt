package io.github.aecsocket.wordbase

import android.util.Log
import android.webkit.JavascriptInterface
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
import uniffi.wordbase_api.ProfileId
import uniffi.wordbase_api.RecordKind
import uniffi.wordbase_api.RecordLookup
import uniffi.wordbase_api.Term

const val TAG = "RecordsView"

@Composable
fun rememberRecordLookup(
    wordbase: Wordbase,
    sentence: String,
    cursor: ULong,
): List<RecordLookup> {
    var records by remember { mutableStateOf(listOf<RecordLookup>()) }

    val app = LocalContext.current.app()
    LaunchedEffect(arrayOf(sentence, cursor, app.dictionaries, app.profiles, app.profileId)) {
        records = wordbase.lookup(
            profileId = app.profileId,
            sentence = sentence,
            cursor = cursor,
            recordKinds = RecordKind.entries,
        )
    }

    return records
}

@Composable
fun RecordsView(
    wordbase: Wordbase,
    records: List<RecordLookup>,
    insets: WindowInsets = WindowInsets(0.dp),
    containerColor: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = contentColorFor(containerColor),
    onAddCard: ((Term) -> Unit)? = null,
    onExit: (() -> Unit)? = null,
) {
    class JsBridge(val onAddCard: (Term) -> Unit) {
        @Suppress("unused") // used by JS
        @JavascriptInterface
        fun addCard(headword: String?, reading: String?) {
            onAddCard(Term(headword = headword, reading = reading))
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
            addCardText = stringResource(R.string.add_card),
            addCardJsFn = "Wordbase.addCard",
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

            it.settings.javaScriptEnabled = true
            onAddCard?.let { onAddCard ->
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

//fun addCard(wordbase: Wordbase, term: Term) {
//    Log.i(TAG, "Adding card for (${term.headword}, ${term.reading})")
//    AddContentApi.getAnkiDroidPackageName(context) // todo checks
//    val anki = AddContentApi(context)
//
//    coroutineScope.launch {
//        wordbase.buildTermNote(
//            profileId = app.profileId,
//            sentence = sente
//        )
//    }
//
//    val deckId = anki.deckList
//        .firstNotNullOfOrNull { (id, name) -> if (name == "Testing") id else null }
//    val modelId = anki.modelList
//        .firstNotNullOfOrNull { (id, name) -> if (name == "Lapis") id else null }
//
//    anki.addNote(modelId!!, deckId!!, arrayOf())
//}

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
