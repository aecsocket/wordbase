package io.github.aecsocket.wordbase

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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.kevinnzou.web.WebView
import com.kevinnzou.web.rememberWebViewNavigator
import com.kevinnzou.web.rememberWebViewStateWithHTMLData
import uniffi.wordbase.Wordbase
import uniffi.wordbase_api.ProfileId
import uniffi.wordbase_api.RecordKind
import uniffi.wordbase_api.RecordLookup

@Composable
fun rememberRecordLookup(
    wordbase: Wordbase,
    profileId: ProfileId,
    sentence: String,
    cursor: ULong,
): List<RecordLookup> {
    var records by remember { mutableStateOf(listOf<RecordLookup>()) }

    val app = LocalContext.current.app()
    LaunchedEffect(arrayOf(profileId, sentence, cursor, app.dictionaries, app.profiles)) {
        records = wordbase.lookup(
            profileId = profileId,
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
    onExit: (() -> Unit)? = null,
) {
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
    val html = wordbase.renderToHtml(records) + "<style>$extraCss</style>"

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
