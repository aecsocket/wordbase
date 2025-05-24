package io.github.aecsocket.wordbase

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.contentColorFor
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import com.kevinnzou.web.WebView
import com.kevinnzou.web.rememberWebViewNavigator
import com.kevinnzou.web.rememberWebViewStateWithHTMLData
import uniffi.wordbase.Wordbase
import uniffi.wordbase_api.RecordKind
import uniffi.wordbase_api.RecordLookup

@Composable
fun LookupView(
    wordbase: Wordbase,
    sentence: String,
    cursor: ULong,
    insets: WindowInsets,
    containerColor: Color = MaterialTheme.colorScheme.surface,
    contentColor: Color = contentColorFor(containerColor),
    onRecords: (List<RecordLookup>) -> Unit = {},
    onExit: (() -> Unit)? = null,
) {
    var html by remember { mutableStateOf<String?>(null) }

    // amazingly, this scales perfectly
    val density = LocalDensity.current
    val layoutDir = LocalLayoutDirection.current
    val extraCss = with(density) {
        """
        :root {
            --bg-color: ${containerColor.css()};
            --fg-color: ${contentColor.css()};
            --accent-color: ${MaterialTheme.colorScheme.primary.css()};
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

    val app = LocalContext.current.app()
    LaunchedEffect(arrayOf(sentence, cursor, app.dictionaries, app.profiles)) {
        val records = wordbase.lookup(
            profileId = 1L,
            sentence = sentence,
            cursor = cursor,
            recordKinds = RecordKind.entries,
        )
        onRecords(records)
        html = wordbase.renderToHtml(records) + "<style>$extraCss</style>"
    }

    html?.let { html ->
        val webViewState = rememberWebViewStateWithHTMLData(html)
        val navigator = rememberWebViewNavigator()
        WebView(
            state = webViewState,
            navigator = navigator,
            modifier = Modifier.fillMaxSize(),
            captureBackPresses = false,
            onCreated = {
                it.settings.javaScriptEnabled = true
                it.settings.allowFileAccess = false
                it.settings.allowContentAccess = false
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
}

fun Color.css() = "rgb(${red * 100}% ${green * 100}% ${blue * 100}%)"
