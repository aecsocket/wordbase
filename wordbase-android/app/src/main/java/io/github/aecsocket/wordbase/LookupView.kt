package io.github.aecsocket.wordbase

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.PaddingValues
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
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import com.kevinnzou.web.WebView
import com.kevinnzou.web.rememberWebViewNavigator
import com.kevinnzou.web.rememberWebViewStateWithHTMLData
import uniffi.wordbase.Wordbase
import uniffi.wordbase_api.RecordKind

@Composable
fun LookupView(
    wordbase: Wordbase,
    query: String,
    padding: PaddingValues = PaddingValues(0.dp),
    containerColor: Color = MaterialTheme.colorScheme.background,
    contentColor: Color = contentColorFor(containerColor),
    headerColor: Color = containerColor,
    onExit: (() -> Unit)? = null,
) {
    var html by remember { mutableStateOf<String?>(null) }

    // amazingly, this scales perfectly
    val padding = """
        <style>
        body {
            padding:
                ${padding.calculateTopPadding().value}px
                ${padding.calculateRightPadding(LayoutDirection.Ltr).value}px
                ${padding.calculateBottomPadding().value}px
                ${padding.calculateLeftPadding(LayoutDirection.Ltr).value}px;
        }
        </style>"""

    val app = LocalContext.current.app()
    LaunchedEffect(arrayOf(query, app.dictionaries, app.profiles)) {
        val records = wordbase.lookup(
            profileId = 1L, sentence = query, cursor = 0UL, recordKinds = RecordKind.entries
        )
        html = wordbase.renderToHtml(
            records = records,
            foregroundColor = contentColor.css(),
            backgroundColor = containerColor.css(),
            headerColor = headerColor.css(),
            accentColor = "#3584e4",
        ) + padding
    }

    html?.let { html ->
        val webViewState = rememberWebViewStateWithHTMLData(html)
        val navigator = rememberWebViewNavigator()
        WebView(
            state = webViewState,
            navigator = navigator,
            modifier = Modifier.fillMaxSize(),
            captureBackPresses = false,
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
