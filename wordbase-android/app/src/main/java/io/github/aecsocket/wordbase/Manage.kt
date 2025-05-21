package io.github.aecsocket.wordbase

import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardColors
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import androidx.core.view.HapticFeedbackConstantsCompat
import androidx.core.view.ViewCompat
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import sh.calvin.reorderable.ReorderableCollectionItemScope
import sh.calvin.reorderable.ReorderableItem
import sh.calvin.reorderable.rememberReorderableLazyListState
import uniffi.wordbase.Dictionary
import uniffi.wordbase.DictionaryKind
import uniffi.wordbase.DictionaryMeta

@Preview(showBackground = true)
@Composable
fun PagePreview() {
    WordbaseTheme {
        ManagePage()
    }
}

val dictionaries = listOf(
    Dictionary(
        id = 1, meta = DictionaryMeta(
            kind = DictionaryKind.YOMITAN,
            name = "Jitendex",
            version = "1.0.0",
            description = "Jitendex foo bar whatever blah. Lorem ipsum dolor sit amet, just fill space so it wraps on a new line",
            url = "https://example.com",
            attribution = "attribution info..."
        ), position = 1
    ), Dictionary(
        id = 2, meta = DictionaryMeta(
            kind = DictionaryKind.YOMITAN,
            name = "JPDB",
            version = "0.1.0",
            description = "desc",
            url = null,
            attribution = null,
        ), position = 2
    )
)

@Composable
fun ManagePage() {
    val view = LocalView.current
    val lazyListState = rememberLazyListState()
    val reorderableLazyListState = rememberReorderableLazyListState(lazyListState) { from, to ->
//        dictionaries = dictionaries.toMutableList().apply {
//            add(to.index - 1, removeAt(from.index - 1))
//        }

        ViewCompat.performHapticFeedback(
            view, HapticFeedbackConstantsCompat.SEGMENT_FREQUENT_TICK
        )
    }

    val context = LocalContext.current
    LazyColumn(
        state = lazyListState, modifier = Modifier.fillMaxSize(), contentPadding = PaddingValues(
            horizontal = 16.dp
        ), verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        item {
            Column {
                Text(
                    "Dictionaries", style = MaterialTheme.typography.headlineLarge
                )
            }
        }

        items(
            dictionaries, key = { meta -> meta.id }) { meta ->
            ReorderableItem(
                reorderableLazyListState, key = meta.id
            ) {
                DictionaryRow(dict = meta)
            }
        }

        item {
            ImportRow(
                state = ImportState.Started(
                    fileName = "dict.zip"
                ),
            )
        }

        item {
            ImportRow(
                state = ImportState.Started(
                    fileName = "dict2.zip"
                ), error = "oh no"
            )
        }

        item {
            ImportRow(
                state = ImportState.ReadMeta(
                    meta = DictionaryMeta(
                        kind = DictionaryKind.YOMITAN,
                        name = "Jitendex",
                        version = "0.2.0",
                        description = null,
                        url = null,
                        attribution = null,
                    ),
                    progress = 0.5f,
                ), error = null
            )
        }

        item {
            val launcher = rememberLauncherForActivityResult(
                contract = ActivityResultContracts.OpenDocument(),
            ) { uri ->
                val uri = uri ?: return@rememberLauncherForActivityResult
                context.contentResolver.openFileDescriptor(uri, "r")?.use { fd ->
                }
            }

            Button(
                onClick = {
                    launcher.launch(arrayOf("application/zip"))
                }, modifier = Modifier.fillMaxWidth()
            ) {
                Text(stringResource(R.string.dictionary_import))
            }
        }
    }
}

@Composable
fun ReorderableCollectionItemScope.DictionaryRow(dict: Dictionary) {
    ExpanderCard(
        modifier = Modifier.fillMaxWidth(),
        titleModifier = Modifier
            .fillMaxWidth()
            .padding(8.dp),
        titleHorizontalArrangement = Arrangement.spacedBy(8.dp),
        titleVerticalAlignment = Alignment.CenterVertically,
        titleContent = {
            val view = LocalView.current
            Icon(
                modifier = Modifier.draggableHandle(onDragStarted = {
                    ViewCompat.performHapticFeedback(
                        view, HapticFeedbackConstantsCompat.GESTURE_START
                    )
                }, onDragStopped = {
                    ViewCompat.performHapticFeedback(
                        view, HapticFeedbackConstantsCompat.GESTURE_END
                    )
                }),
                painter = painterResource(R.drawable.outline_drag_indicator_24),
                contentDescription = null
            )

            Column(
                modifier = Modifier.weight(1f),
            ) {
                Text(
                    text = dict.meta.name, style = MaterialTheme.typography.bodyLarge
                )

                dict.meta.version?.let { version ->
                    Text(
                        text = version, style = MaterialTheme.typography.bodyMedium
                    )
                }
            }

            Switch(
                checked = true, onCheckedChange = {})
        }) {
        DictionaryMetaColumn(
            meta = dict.meta,
            onSetSortingDict = {},
            onDelete = {},
        )
    }
}

sealed class ImportState {
    data class Started(
        val fileName: String,
    ) : ImportState()

    data class ReadMeta(
        val meta: DictionaryMeta,
        val progress: Float,
    ) : ImportState()
}

@Composable
fun ImportRow(
    state: ImportState,
    error: String? = null,
) {
    ExpanderCard(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = if (error == null) {
                Color.Unspecified
            } else {
                MaterialTheme.colorScheme.errorContainer
            }
        ),

        titleModifier = Modifier
            .fillMaxWidth()
            .padding(8.dp),
        titleHorizontalArrangement = Arrangement.spacedBy(8.dp),
        titleVerticalAlignment = Alignment.CenterVertically,
        titleContent = {
            Box {
                val (progressAlpha, iconAlpha) = if (error == null) {
                    Pair(1f, 0f)
                } else {
                    Pair(0f, 1f)
                }

                when (state) {
                    is ImportState.Started -> {
                        CircularProgressIndicator(
                            modifier = Modifier.alpha(progressAlpha)
                        )
                    }

                    is ImportState.ReadMeta -> {
                        CircularProgressIndicator(
                            modifier = Modifier.alpha(progressAlpha), progress = { state.progress })
                    }
                }

                Icon(
                    modifier = Modifier
                        .align(Alignment.Center)
                        .alpha(iconAlpha),
                    imageVector = Icons.Default.Warning,
                    contentDescription = stringResource(R.string.dictionary_import_failed)
                )
            }

            val title = when (state) {
                is ImportState.Started -> {
                    state.fileName
                }

                is ImportState.ReadMeta -> {
                    state.meta.name
                }
            }
            val subtitle = if (error == null) {
                when (state) {
                    is ImportState.Started -> null
                    is ImportState.ReadMeta -> state.meta.version
                }
            } else {
                stringResource(R.string.dictionary_import_failed)
            }

            Column(
                modifier = Modifier.weight(1f),
            ) {
                Text(
                    text = title, style = MaterialTheme.typography.bodyLarge
                )

                AnimatedVisibility(visible = subtitle != null) {
                    subtitle?.let { subtitle ->
                        Text(
                            text = subtitle, style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }
            }

            IconButton(
                onClick = {}) {
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = stringResource(R.string.dictionary_import_cancel)
                )
            }
        }) {
        DictionaryMetaColumn(
            meta = when (state) {
                is ImportState.Started -> null
                is ImportState.ReadMeta -> state.meta
            }, top = {
                if (error != null) {
                    DictionaryMetaItem(
                        key = stringResource(R.string.dictionary_import_error), value = error
                    )
                }
            })
    }
}

@Composable
fun DictionaryMetaColumn(
    meta: DictionaryMeta? = null,
    top: @Composable () -> Unit = {},
    onSetSortingDict: (() -> Unit)? = null,
    onDelete: (() -> Unit)? = null,
) {
    Column(
        modifier = Modifier.padding(8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        top()

        meta?.kind?.let { kind ->
            DictionaryMetaItem(
                key = stringResource(R.string.dictionary_format), value = stringResource(
                    when (kind) {
                        DictionaryKind.YOMITAN -> R.string.dictionary_format_yomitan
                        DictionaryKind.YOMICHAN_AUDIO -> R.string.dictionary_format_yomichan_audio
                    }
                )
            )
        }

        meta?.description?.let { description ->
            DictionaryMetaItem(
                key = stringResource(R.string.dictionary_description), value = description
            )
        }

        meta?.attribution?.let { attribution ->
            DictionaryMetaItem(
                key = stringResource(R.string.dictionary_attribution), value = attribution
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
            verticalAlignment = Alignment.CenterVertically
        ) {
            val context = LocalContext.current

            onSetSortingDict?.let { onSetSortingDict ->
                IconButton(onClick = onSetSortingDict) {
                    Icon(
                        painter = painterResource(R.drawable.outline_sort_24),
                        contentDescription = stringResource(R.string.dictionary_set_sorting)
                    )
                }
            }

            meta?.url?.let { url ->
                IconButton(
                    onClick = {
                        val intent = Intent(Intent.ACTION_VIEW, url.toUri())
                        context.startActivity(intent)
                    }) {
                    Icon(
                        painter = painterResource(R.drawable.outline_globe_24),
                        contentDescription = stringResource(R.string.dictionary_visit_website)
                    )
                }
            }

            onDelete?.let { onDelete ->
                IconButton(onClick = onDelete) {
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
            key, style = MaterialTheme.typography.labelMedium
        )

        Text(value)
    }
}

@Composable
fun ExpanderCard(
    modifier: Modifier = Modifier,
    colors: CardColors = CardDefaults.cardColors(),
    titleContent: @Composable RowScope.() -> Unit,
    titleModifier: Modifier = Modifier,
    titleHorizontalArrangement: Arrangement.Horizontal = Arrangement.Start,
    titleVerticalAlignment: Alignment.Vertical = Alignment.Top,
    content: @Composable ColumnScope.() -> Unit
) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    Card(modifier = modifier, colors = colors) {
        Column {
            Column(
                modifier = Modifier.clickable { expanded = !expanded }) {
                Row(
                    modifier = titleModifier,
                    horizontalArrangement = titleHorizontalArrangement,
                    verticalAlignment = titleVerticalAlignment
                ) {
                    titleContent()

                    Icon(
                        imageVector = if (expanded) {
                            Icons.Default.KeyboardArrowUp
                        } else {
                            Icons.Default.KeyboardArrowDown
                        }, contentDescription = null
                    )
                }
            }

            AnimatedVisibility(visible = expanded) {
                content()
            }
        }
    }
}
