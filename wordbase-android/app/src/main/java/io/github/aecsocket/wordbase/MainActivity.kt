package io.github.aecsocket.wordbase

import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.content.res.Resources
import android.net.Uri
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
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
//            Wordbase = uniffi.wordbase_engine.engine(filesDir.absolutePath)
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
        val html = LocalContext.current.resources.openRawResource(R.raw.records)
            .bufferedReader()
            .use { it.readText() }

        val webViewState = rememberWebViewStateWithHTMLData(html)
        WebView(
            state = webViewState,
            modifier = Modifier.fillMaxSize(),
            onCreated = {
                it.setBackgroundColor(0)
            }
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


