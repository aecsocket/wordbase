package io.github.aecsocket.wordbase

import android.content.Context
import android.content.ContextWrapper
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.BottomSheetScaffold
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults.topAppBarColors
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import androidx.compose.ui.unit.dp
import androidx.core.view.HapticFeedbackConstantsCompat
import androidx.core.view.ViewCompat
import androidx.lifecycle.lifecycleScope
import androidx.window.core.layout.WindowWidthSizeClass
import com.kevinnzou.web.WebView
import com.kevinnzou.web.rememberWebViewStateWithHTMLData
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import kotlinx.coroutines.launch
import sh.calvin.reorderable.ReorderableCollectionItemScope
import sh.calvin.reorderable.ReorderableItem
import sh.calvin.reorderable.rememberReorderableLazyListState
import uniffi.wordbase.Dictionary
import uniffi.wordbase.DictionaryKind
import uniffi.wordbase.DictionaryMeta
import uniffi.wordbase.RecordKind

lateinit var Wordbase: uniffi.wordbase_engine.Engine

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        lifecycleScope.launch {
            Wordbase = uniffi.wordbase_engine.engine(filesDir.absolutePath)
        }
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
    if (currentWindowAdaptiveInfo().windowSizeClass.windowWidthSizeClass == WindowWidthSizeClass.COMPACT) {
        BottomSheetScaffold(
            topBar = {
                TopAppBar(
                    colors = topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer,
                        titleContentColor = MaterialTheme.colorScheme.primary,
                    ),
                    title = {
                        SearchBar()
                    }
                )
            },
            sheetContent = {
                ManagePage()
            },
            sheetPeekHeight = 96.dp
        ) { padding ->
            SearchPage(padding = padding)
        }
    } else {
        val coroutineScope = rememberCoroutineScope()
        val drawerState = rememberDrawerState(DrawerValue.Closed)
        ModalNavigationDrawer(
            drawerState = drawerState,
            drawerContent = {
                ModalDrawerSheet {
                    ManagePage()
                }
            }
        ) {
            Scaffold(
                topBar = {
                    TopAppBar(
                        colors = topAppBarColors(
                            containerColor = MaterialTheme.colorScheme.primaryContainer,
                            titleContentColor = MaterialTheme.colorScheme.primary,
                        ),
                        title = {
                            SearchBar()
                        },
                        navigationIcon = {
                            IconButton(
                                onClick = {
                                    coroutineScope.launch {
                                        drawerState.open()
                                    }
                                }
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Menu,
                                    contentDescription = stringResource(R.string.open_menu)
                                )
                            }
                        }
                    )
                }
            ) { padding ->
                SearchPage(padding = padding)
            }
        }
    }
}

@Composable
fun SearchBar() {
    var query by remember { mutableStateOf("") }
    TextField(
        value = query,
        onValueChange = { query = it },
        singleLine = true,
        leadingIcon = {
            Icon(imageVector = Icons.Default.Search, contentDescription = null)
        },
        modifier = Modifier
            .fillMaxWidth()
            .padding(8.dp)
    )
}

@Composable
fun SearchPage(padding: PaddingValues) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .imePadding()
            .padding(padding)
    ) {
        val webViewState = rememberWebViewStateWithHTMLData(
            """
            <h1>Hello world!</h1>
            ${RecordKind.entries}
            <p>some text</p>
            """.trimIndent()
        )
        WebView(
            state = webViewState,
            modifier = Modifier.fillMaxSize(),
            onCreated = {
                it.setBackgroundColor(0)
            }
        )
    }
}

@Composable
fun ManagePage() {
    var dictionaries by remember {
        mutableStateOf(
            listOf(
                Dictionary(
                    id = 1,
                    meta = DictionaryMeta(
                        kind = DictionaryKind.YOMITAN,
                        name = "Jitendex",
                        version = "1.0.0",
                        description = "Jitendex foo bar whatever blah. Lorem ipsum dolor sit amet, just fill space so it wraps on a new line",
                        url = "https://example.com",
                        attribution = "attribution info..."
                    ),
                    position = 1
                ),
                Dictionary(
                    id = 2,
                    meta = DictionaryMeta(
                        kind = DictionaryKind.YOMITAN,
                        name = "JPDB",
                        version = "0.1.0",
                        description = "desc",
                        url = null,
                        attribution = null,
                    ),
                    position = 2
                )
            )
        )
    }

    val view = LocalView.current
    val lazyListState = rememberLazyListState()
    val reorderableLazyListState = rememberReorderableLazyListState(lazyListState) { from, to ->
        dictionaries = dictionaries.toMutableList().apply {
            add(to.index - 1, removeAt(from.index - 1))
        }

        ViewCompat.performHapticFeedback(
            view,
            HapticFeedbackConstantsCompat.SEGMENT_FREQUENT_TICK
        )
    }

    LazyColumn(
        state = lazyListState,
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(
            horizontal = 16.dp
        ),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        item {
            Text(
                "Dictionaries",
                style = MaterialTheme.typography.headlineLarge
            )
        }

        items(
            dictionaries,
            key = { meta -> meta.id }
        ) { meta ->
            ReorderableItem(
                reorderableLazyListState,
                key = meta.id
            ) {
                DictionaryRow(dict = meta)
            }
        }

        item {
            val launcher = rememberLauncherForActivityResult(
                contract = ActivityResultContracts.OpenDocument(),
            ) { result ->

            }

            Button(
                onClick = {
                    launcher.launch(arrayOf("application/zip"))
                },
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(stringResource(R.string.dictionary_import))
            }
        }
    }
}

@PreviewScreenSizes
@Preview
@Composable
fun UiPreview() {
    WordbaseTheme {
        Ui()
    }
}

@Composable
fun ReorderableCollectionItemScope.DictionaryRow(dict: Dictionary) {
    ExpanderCard(
        titleContent = {
            Row(
                modifier = Modifier.padding(8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                val view = LocalView.current
                Icon(
                    modifier = Modifier.draggableHandle(
                        onDragStarted = {
                            ViewCompat.performHapticFeedback(
                                view,
                                HapticFeedbackConstantsCompat.GESTURE_START
                            )
                        },
                        onDragStopped = {
                            ViewCompat.performHapticFeedback(
                                view,
                                HapticFeedbackConstantsCompat.GESTURE_END
                            )
                        }
                    ),
                    painter = painterResource(R.drawable.outline_drag_indicator_24),
                    contentDescription = null
                )

                Column {
                    Text(
                        dict.meta.name,
                        style = MaterialTheme.typography.bodyLarge
                    )

                    dict.meta.version?.let { version ->
                        Text(
                            version,
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Switch(
                        checked = true,
                        onCheckedChange = {}
                    )
                }
            }
        }
    ) {
        Column(
            modifier = Modifier.padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            dict.meta.description?.let { description ->
                DictionaryMetaItem(
                    key = stringResource(R.string.dictionary_description),
                    value = description
                )
            }

            dict.meta.attribution?.let { attribution ->
                DictionaryMetaItem(
                    key = stringResource(R.string.dictionary_attribution),
                    value = attribution
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
                verticalAlignment = Alignment.CenterVertically
            ) {
                IconButton(
                    onClick = {}
                ) {
                    Icon(
                        painter = painterResource(R.drawable.outline_sort_24),
                        contentDescription = stringResource(R.string.dictionary_set_sorting)
                    )
                }

                IconButton(
                    onClick = {}
                ) {
                    Icon(
                        painter = painterResource(R.drawable.outline_globe_24),
                        contentDescription = stringResource(R.string.dictionary_visit_website)
                    )
                }

                IconButton(
                    onClick = {}
                ) {
                    Icon(
                        imageVector = Icons.Default.Delete,
                        contentDescription = stringResource(R.string.dictionary_delete)
                    )
                }
            }
        }
    }
}

@Composable
fun DictionaryMetaItem(key: String, value: String) {
    Column {
        Text(
            key,
            style = MaterialTheme.typography.labelMedium
        )

        Text(value)
    }
}

@Composable
fun ExpanderCard(
    modifier: Modifier = Modifier,
    titleContent: @Composable ColumnScope.() -> Unit,
    content: @Composable ColumnScope.() -> Unit
) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    Card(modifier = modifier) {
        Column {
            Column(
                modifier = Modifier
                    .clickable { expanded = !expanded }
            ) {
                titleContent()
            }

            AnimatedVisibility(visible = expanded) {
                content()
            }
        }
    }
}
