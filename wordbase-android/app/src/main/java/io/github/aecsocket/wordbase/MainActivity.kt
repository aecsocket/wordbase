package io.github.aecsocket.wordbase

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.BottomSheetScaffold
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults.topAppBarColors
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.window.core.layout.WindowWidthSizeClass
import com.kevinnzou.web.WebView
import com.kevinnzou.web.rememberWebViewStateWithHTMLData
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import kotlinx.coroutines.launch
import uniffi.wordbase.Engine
import uniffi.wordbase_api.RecordKind

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            WordbaseTheme {
                Ui()
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Ui() {
    var query by remember { mutableStateOf("") }
    if (currentWindowAdaptiveInfo().windowSizeClass.windowWidthSizeClass == WindowWidthSizeClass.COMPACT) {
        BottomSheetScaffold(
            topBar = {
                TopAppBar(
                    colors = topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer,
                        titleContentColor = MaterialTheme.colorScheme.primary,
                    ), title = {
                        SearchBar(
                            query = query, onQueryChange = { query = it })
                    })
            }, sheetContent = {
                ManagePage()
            }, sheetPeekHeight = 96.dp
        ) { padding ->
            SearchPage(padding = padding, query = query)
        }
    } else {
        val coroutineScope = rememberCoroutineScope()
        val drawerState = rememberDrawerState(DrawerValue.Closed)
        ModalNavigationDrawer(
            drawerState = drawerState, drawerContent = {
                ModalDrawerSheet {
                    ManagePage()
                }
            }) {
            Scaffold(
                topBar = {
                    TopAppBar(
                        colors = topAppBarColors(
                            containerColor = MaterialTheme.colorScheme.primaryContainer,
                            titleContentColor = MaterialTheme.colorScheme.primary,
                        ), title = {
                            SearchBar(
                                query = query, onQueryChange = { query = it })
                        }, navigationIcon = {
                            IconButton(
                                onClick = {
                                    coroutineScope.launch {
                                        drawerState.open()
                                    }
                                }) {
                                Icon(
                                    imageVector = Icons.Default.Menu,
                                    contentDescription = stringResource(R.string.open_menu)
                                )
                            }
                        })
                }) { padding ->
                SearchPage(padding = padding, query = query)
            }
        }
    }
}

@Composable
fun SearchBar(query: String, onQueryChange: (String) -> Unit) {
    TextField(
        value = query,
        onValueChange = onQueryChange,
        singleLine = true,
        leadingIcon = {
            Icon(imageVector = Icons.Default.Search, contentDescription = null)
        },
        modifier = Modifier
            .fillMaxWidth()
            .padding(8.dp),
        textStyle = MaterialTheme.typography.bodySmall
    )
}

@Composable
fun SearchPage(padding: PaddingValues, query: String) {
    val bottomPadding = padding.calculateBottomPadding().value
    // amazingly, this scales perfectly
    val paddingCss = "<style>body { padding: 0 0 ${bottomPadding}px 0; }</style>"

    Column(
        modifier = Modifier.fillMaxSize()
    ) {
        val context = LocalContext.current
        var wordbase by remember { mutableStateOf<Engine?>(null) }

        LaunchedEffect(Unit) {
            wordbase = context.wordbase()
        }

        var html by remember { mutableStateOf("waiting... TODO") }

        val files = LocalContext.current.filesDir?.list()

        Text(text = "f = ${files.contentToString()}")

        wordbase?.let { wordbase ->
            Text(text = "dicts = ${wordbase.dictionaries().map { it.meta.name }}")

            Text(text = "profiles = ${wordbase.profiles()}")

            LaunchedEffect(query) {
                val records = wordbase.lookup(
                    profileId = 1L, sentence = query, cursor = 0UL, recordKinds = RecordKind.entries
                )
                html = "hello!" + wordbase.renderToHtml(
                    records = records,
                    accentColorR = 0x35U,
                    accentColorG = 0x84U,
                    accentColorB = 0xe4U,
                ) + paddingCss
            }
        }

        val webViewState = rememberWebViewStateWithHTMLData(html)
        WebView(
            state = webViewState,
            modifier = Modifier.fillMaxSize(),
        )
    }
}


//@PreviewScreenSizes
@Preview
@Composable
fun UiPreview() {
    WordbaseTheme {
        Ui()
    }
}


