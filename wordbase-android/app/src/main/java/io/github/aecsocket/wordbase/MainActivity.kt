package io.github.aecsocket.wordbase

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.displayCutoutPadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.BottomSheetScaffold
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.SearchBar
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.intl.LocaleList
import androidx.compose.ui.text.style.TextAlign
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
                AppUi()
            }
        }
    }
}

//@PreviewScreenSizes
@Preview(showSystemUi = true)
@Composable
fun PreviewUi() {
    WordbaseTheme {
        Ui(
            manageContent = { modifier ->
                PreviewManagePage(modifier = modifier)
            }
        )
    }
}

@Composable
fun AppUi() {
    WordbaseTheme {
        Ui(
            manageContent = { modifier ->
                AppManagePage(modifier = modifier)
            }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Ui(manageContent: @Composable (Modifier) -> Unit) {
    var query by rememberSaveable { mutableStateOf("") }

    val searchFocusRequester = remember { FocusRequester() }
    val inputField = @Composable {
        SearchBarInputField(
            query = query,
            onQueryChange = { query = it },
            onSearch = {},
            expanded = false,
            onExpandedChange = {},
            placeholder = {
                Text(text = stringResource(R.string.search_title))
            },
            leadingIcon = {
                Icon(
                    imageVector = Icons.Default.Search,
                    contentDescription = null,
                )
            },
            trailingIcon = {
                if (query.isNotEmpty()) {
                    IconButton(
                        onClick = { query = "" },
                    ) {
                        Icon(
                            imageVector = Icons.Default.Clear,
                            contentDescription = stringResource(R.string.search_clear),
                        )
                    }
                }
            },
            focusRequester = searchFocusRequester,
            keyboardOptions = KeyboardOptions(
                imeAction = ImeAction.Search,
                keyboardType = KeyboardType.Text,
                hintLocales = LocaleList("ja-JP"), // TODO
            ),
        )
    }

    LaunchedEffect(Unit) {
        searchFocusRequester.requestFocus()
    }

    if (currentWindowAdaptiveInfo().windowSizeClass.windowWidthSizeClass == WindowWidthSizeClass.COMPACT) {
        BottomSheetScaffold(
            sheetContent = {
                manageContent(Modifier.navigationBarsPadding())
            },
            sheetPeekHeight = 96.dp
        ) { padding ->
            Column {
                Surface {
                    SearchBar(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 4.dp),
                        inputField = inputField,
                        expanded = false,
                        onExpandedChange = {},
                    ) {}
                }

                val layoutDir = LocalLayoutDirection.current
                SearchPage(
                    padding = padding,
                    insets = WindowInsets(
                        left = padding.calculateLeftPadding(layoutDir),
                        right = padding.calculateRightPadding(layoutDir),
                        top = padding.calculateTopPadding(),
                        bottom = padding.calculateBottomPadding()
                    ),
                    query = query,
                )
            }
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
                    manageContent(Modifier.displayCutoutPadding())
                }
            }
        ) {
            Surface {
                Column(modifier = Modifier.statusBarsPadding()) {
                    Row(
                        modifier = Modifier
                            .padding(horizontal = 16.dp)
                            .displayCutoutPadding(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        IconButton(
                            onClick = {
                                coroutineScope.launch {
                                    drawerState.open()
                                }
                            }
                        ) {
                            Icon(
                                imageVector = Icons.Default.Menu,
                                contentDescription = stringResource(R.string.open_menu),
                            )
                        }

                        SearchBar(
                            modifier = Modifier.weight(1f),
                            inputField = inputField,
                            expanded = false,
                            onExpandedChange = {},
                        ) {}
                    }

                    SearchPage(
                        padding = PaddingValues(0.dp),
                        insets = WindowInsets.displayCutout,
                        query = query
                    )
                }
            }
        }
    }
}

@Composable
fun SearchPage(
    padding: PaddingValues,
    insets: WindowInsets,
    query: String
) {
    var wordbase by rememberWordbase()

    wordbase?.let { wordbase ->
        val activity = LocalActivity.current

        val entries = rememberLookup(
            wordbase = wordbase,
            sentence = query,
            cursor = 0UL,
        )

        if (entries.isEmpty()) {
            if (query.isNotEmpty()) {
                NoRecordsView()
            }
        } else {
            RecordsView(
                wordbase = wordbase,
                sentence = query,
                cursor = 0UL,
                entries = entries,
                insets = insets,
                onExit = { activity?.finish() }
            )
        }
    } ?: run {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            CircularProgressIndicator()

            Text(
                text = stringResource(R.string.loading_title),
                style = MaterialTheme.typography.headlineMedium
            )

            Text(
                text = stringResource(R.string.loading_body),
                textAlign = TextAlign.Center,
            )
        }
    }
}
