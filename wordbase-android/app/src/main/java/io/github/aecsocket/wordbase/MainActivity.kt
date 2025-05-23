package io.github.aecsocket.wordbase

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
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
import androidx.compose.material3.TextField
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults.topAppBarColors
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.rememberBottomSheetScaffoldState
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.window.core.layout.WindowWidthSizeClass
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import kotlinx.coroutines.launch

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
    var query by rememberSaveable { mutableStateOf("") }
    val headerColor = MaterialTheme.colorScheme.surfaceContainer

    val topAppBarColors = topAppBarColors(containerColor = headerColor)
    if (currentWindowAdaptiveInfo().windowSizeClass.windowWidthSizeClass == WindowWidthSizeClass.COMPACT) {
        BottomSheetScaffold(
            topBar = {
                TopAppBar(
                    colors = topAppBarColors,
                    title = {
                        SearchBar(query = query, onQueryChange = { query = it })
                    }
                )
            },
            sheetContent = {
                AppManagePage(
                    modifier = Modifier.navigationBarsPadding()
                )
            },
            sheetPeekHeight = 96.dp
        ) { padding ->
            SearchPage(padding = padding, query = query, headerColor = headerColor)
        }
    } else {
        val coroutineScope = rememberCoroutineScope()
        val drawerState = rememberDrawerState(DrawerValue.Closed)
        ModalNavigationDrawer(
            drawerState = drawerState,
            // allow closing with a swipe, but not opening
            gesturesEnabled = drawerState.isOpen,
            drawerContent = {
                ModalDrawerSheet {
                    AppManagePage()
                }
            }) {
            Scaffold(
                topBar = {
                    TopAppBar(
                        colors = topAppBarColors,
                        title = {
                            SearchBar(
                                query = query, onQueryChange = { query = it })
                        },
                        navigationIcon = {
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
                SearchPage(padding = padding, query = query, headerColor = headerColor)
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
fun SearchPage(
    padding: PaddingValues,
    query: String,
    headerColor: Color
) {
    var foo by remember { mutableStateOf("foo") }
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(top = padding.calculateTopPadding())
    ) {
        TextField(
            value = foo,
            onValueChange = { foo = it }
        )

        var wordbase by rememberWordbase()

        wordbase?.let { wordbase ->
            val activity = LocalActivity.current
            LookupView(
                wordbase = wordbase,
                query = query,
                padding = PaddingValues(
                    bottom = padding.calculateBottomPadding(),
                ),
                headerColor = headerColor,
                onExit = {
                    activity?.finish()
                }
            )
        }
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


