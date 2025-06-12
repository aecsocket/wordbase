@file:OptIn(ExperimentalUuidApi::class)

package io.github.aecsocket.wordbase

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.icu.number.NumberFormatter
import android.icu.number.Precision
import android.icu.text.NumberFormat
import android.net.Uri
import android.provider.DocumentsContract
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.AlertDialogDefaults
import androidx.compose.material3.BasicAlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardColors
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import androidx.core.view.HapticFeedbackConstantsCompat
import androidx.core.view.ViewCompat
import com.ichi2.anki.api.AddContentApi
import io.github.aecsocket.wordbase.ui.theme.WordbaseTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import sh.calvin.reorderable.ReorderableCollectionItemScope
import sh.calvin.reorderable.ReorderableItem
import sh.calvin.reorderable.rememberReorderableLazyListState
import uniffi.wordbase.ImportDictionaryCallback
import uniffi.wordbase.ImportEvent
import uniffi.wordbase.ImportProgress
import uniffi.wordbase_api.Dictionary
import uniffi.wordbase_api.DictionaryKind
import uniffi.wordbase_api.DictionaryMeta
import uniffi.wordbase_api.Profile
import java.util.Locale
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

@Preview(showBackground = true)
@Composable
fun PreviewManagePage(modifier: Modifier = Modifier) {
    WordbaseTheme {
        fun DictionaryMeta(name: String) = DictionaryMeta(
            kind = DictionaryKind.YOMITAN,
            name = name,
            version = "1.0.0",
            description = "my description",
            url = "https://example.com",
            attribution = "some attribution info"
        )

        var dictionaries by remember {
            mutableStateOf(
                listOf(
                    Dictionary(
                        id = 1L,
                        meta = DictionaryMeta("Dict 1"),
                        position = 1
                    ),
                    Dictionary(
                        id = 2L,
                        meta = DictionaryMeta("Dict 2"),
                        position = 2
                    )
                )
            )
        }

        var profile by remember {
            mutableStateOf(
                Profile(
                    id = 1L,
                    name = null,
                    sortingDictionary = 1L,
                    fontFamily = null,
                    ankiDeck = null,
                    ankiNoteType = null,
                    enabledDictionaries = listOf(2L)
                )
            )
        }

        var ankiDeck by remember { mutableStateOf("Mining") }
        var ankiModel by remember { mutableStateOf("Lapis") }

        ManagePage(
            modifier = modifier,
            enabled = true,
            dictionaries = dictionaries,
            dictionaryImports = listOf(
                DictionaryImport.Started(
                    id = Uuid.random(),
                    fileName = "dict.zip",
                ),
                DictionaryImport.ReadMeta(
                    id = Uuid.random(),
                    meta = DictionaryMeta("Another Dict"),
                    progress = ImportProgress(
                        frac = 0.25,
                    ),
                ),
            ),
            profile = profile,
            onDictionaryReorder = { from, to ->
                val fromOld = from.position
                from.position = to.position
                to.position = fromOld
                dictionaries = dictionaries.sortedBy { it.position }
            },
            onDictionarySortingSet = { dictionary ->
                profile = profile.copy(
                    sortingDictionary = dictionary.id
                )
            },
            onDictionarySortingUnset = {
                profile = profile.copy(
                    sortingDictionary = null
                )
            },
            onDictionaryDelete = { dictionary ->
                dictionaries = dictionaries.toMutableList().apply {
                    removeIf { it.id == dictionary.id }
                }
            },
            onDictionaryEnabledChange = { dictionary, enabled ->
                profile = profile.copy(
                    enabledDictionaries = profile.enabledDictionaries.toMutableList().apply {
                        if (enabled) {
                            add(dictionary.id)
                        } else {
                            removeIf { it == dictionary.id }
                        }
                    }
                )
            },
            onDictionaryImport = {},
            anki = { AnkiPagePreview() }
        )
    }
}

@Composable
fun AppManagePage(modifier: Modifier = Modifier) {
    val wordbaseState by rememberWordbase()
    val wordbase = wordbaseState ?: return

    val coroutineScope = rememberCoroutineScope()
    val app = LocalContext.current.app()
    val profile = app.profiles.values.first() // TODO switchable profile

    // Some actions, like deleting a dictionary, may take a long time
    // during this, if the user performs another action, it may fail
    // because the database will be locked for too long.
    // To avoid this, we use `locked` to lock out the user from interactions
    // while one of these long-running operations is happening.
    // Note we only do this on ops we KNOW will take a long time,
    // to avoid locking the UI on every tiny change.
    // TODO: This is effectively a hand-rolled mutex. Can we use an actual mutex somehow?
    var locked by remember { mutableStateOf(false) }

    var importState by remember { mutableStateOf<DictionaryImport?>(null) }

    val context = LocalContext.current
    val importLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        val uri = uri ?: return@rememberLauncherForActivityResult

        coroutineScope.launch {
            locked = true
            val importId = Uuid.random()
            var meta: DictionaryMeta? = null

            val fileName = try {
                Uri.decode(DocumentsContract.getDocumentId(uri))
                    .substringAfterLast('/')
            } catch (_: IllegalArgumentException) {
                uri.toString()
            }

            importState = DictionaryImport.Started(
                id = importId,
                fileName = fileName,
            )

            val callback = object : ImportDictionaryCallback {
                @SuppressLint("Recycle") // we are passing the fd to native code
                override fun openArchiveFile(): Int {
                    val fd = context.contentResolver.openFileDescriptor(uri, "r")
                        ?: throw Exception("failed to open file")
                    return fd.detachFd()
                }

                override fun onEvent(event: ImportEvent) {
                    when (event) {
                        is ImportEvent.DeterminedKind -> {}
                        is ImportEvent.Done -> {}
                        is ImportEvent.ParsedMeta -> {
                            meta = event.v1
                            importState = DictionaryImport.ReadMeta(
                                id = importId,
                                meta = meta,
                                progress = ImportProgress(frac = 0.0),
                            )
                        }

                        is ImportEvent.Progress -> {
                            meta?.let { meta ->
                                importState = DictionaryImport.ReadMeta(
                                    id = importId,
                                    meta = meta,
                                    progress = event.v1,
                                )
                            }
                        }
                    }
                }
            }

            try {
                app.writeToWordbase(wordbase) {
                    val dictId = wordbase.importDictionary(callback)
                    wordbase.enableDictionary(profile.id, dictId)
                }
                importState = null
            } catch (ex: Exception) {
                importState = importState?.withError(ex.toString())
            } finally {
                locked = false
            }
        }
    }

    ManagePage(
        modifier = modifier,
        enabled = !locked,
        profile = profile,
        dictionaries = app.dictionaries.values.sortedBy { it.position },
        dictionaryImports = importState?.let { listOf(it) } ?: listOf(),
        onDictionaryReorder = { from, to ->
            app.writeToWordbase(wordbase) {
                wordbase.swapDictionaryPositions(from.id, to.id)
            }
        },
        onDictionarySortingSet = { dict ->
            coroutineScope.launch {
                app.writeToWordbase(wordbase) {
                    wordbase.setSortingDictionary(profile.id, dict.id)
                }
            }
        },
        onDictionarySortingUnset = {
            coroutineScope.launch {
                app.writeToWordbase(wordbase) {
                    wordbase.setSortingDictionary(profile.id, null)
                }
            }
        },
        onDictionaryDelete = { dict ->
            coroutineScope.launch {
                locked = true
                try {
                    app.writeToWordbase(wordbase) {
                        wordbase.removeDictionary(dict.id)
                    }
                } finally {
                    locked = false
                }
            }
        },
        onDictionaryEnabledChange = { dictionary, enabled ->
            coroutineScope.launch {
                app.writeToWordbase(wordbase) {
                    if (enabled) {
                        wordbase.enableDictionary(profile.id, dictionary.id)
                    } else {
                        wordbase.disableDictionary(profile.id, dictionary.id)
                    }
                }
            }
        },
        onDictionaryImport = {
            importLauncher.launch(
                arrayOf(
                    "application/zip",
                    "application/x-tar",
                    "application/x-xz"
                )
            )
        },
        anki = { AnkiPageApp() },
    )
}

@Composable
fun ManagePage(
    modifier: Modifier = Modifier,
    profile: Profile,
    enabled: Boolean = true,
    dictionaries: List<Dictionary>,
    dictionaryImports: List<DictionaryImport>,
    onDictionaryReorder: suspend CoroutineScope.(Dictionary, Dictionary) -> Unit,
    onDictionarySortingSet: (Dictionary) -> Unit,
    onDictionarySortingUnset: () -> Unit,
    onDictionaryDelete: (Dictionary) -> Unit,
    onDictionaryEnabledChange: (Dictionary, Boolean) -> Unit,
    onDictionaryImport: () -> Unit,
    anki: @Composable () -> Unit,
) {
    val view = LocalView.current
    val context = LocalContext.current
    val anki =
        if (AddContentApi.getAnkiDroidPackageName(context) == null) null else AddContentApi(context);
    val lazyListState = rememberLazyListState()
    val reorderableLazyListState = rememberReorderableLazyListState(lazyListState) { from, to ->
        val from = dictionaries[from.index - 1]
        val to = dictionaries[to.index - 1]
        onDictionaryReorder(from, to)

        ViewCompat.performHapticFeedback(
            view, HapticFeedbackConstantsCompat.SEGMENT_FREQUENT_TICK
        )
    }

    LazyColumn(
        state = lazyListState,
        modifier = modifier,
        contentPadding = PaddingValues(
            horizontal = 16.dp,
        ),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        item {
            Row(
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = stringResource(R.string.section_dictionaries),
                    style = MaterialTheme.typography.headlineLarge,
                    modifier = Modifier.weight(1f),
                )

                CircularProgressIndicator(
                    modifier = Modifier
                        .padding(4.dp)
                        .height(IntrinsicSize.Min)
                        .alpha(if (enabled) 0.0f else 1.0f)
                )
            }
        }

        items(
            dictionaries,
            key = { it.id }
        ) { dictionary ->
            ReorderableItem(
                state = reorderableLazyListState,
                key = dictionary.id,
                enabled = enabled,
            ) {
                DictionaryRow(
                    dictionary = dictionary,
                    profile = profile,
                    onSortingSet = {
                        onDictionarySortingSet(dictionary)
                    },
                    onSortingUnset = {
                        onDictionarySortingUnset()
                    },
                    onDelete = {
                        onDictionaryDelete(dictionary)
                    },
                    onEnabledChange = { enabled ->
                        onDictionaryEnabledChange(dictionary, enabled)
                    },
                    enabled = enabled,
                )
            }
        }

        items(
            dictionaryImports,
            key = { it.id }
        ) { import ->
            ImportRow(import)
        }

        item {
            Button(
                onClick = onDictionaryImport,
                modifier = Modifier.fillMaxWidth(),
                enabled = enabled,
            ) {
                Text(stringResource(R.string.dictionary_import))
            }
        }

        item {
            Text(
                text = stringResource(R.string.manage_anki),
                style = MaterialTheme.typography.headlineLarge,
            )
        }

        item {
            anki()
        }

        item {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text(
                    text = "EARLY PRE RELEASE BUILD",
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.headlineMedium,
                    textAlign = TextAlign.Center,
                )

                Text(
                    text = "App functions may break. Your database may get corrupted. " +
                            "You may have to reimport your dictionaries a lot, " +
                            "or even delete and reinstall the app. " +
                            "DO NOT delete the original dictionary zips!",
                    textAlign = TextAlign.Center,
                )

                Spacer(modifier = Modifier.height(16.dp))

                val context = LocalContext.current
                val version =
                    context.packageManager.getPackageInfo(context.packageName, 0)?.versionName

                Text(
                    text = "Current app version: $version",
                    textAlign = TextAlign.Center,
                )

                Spacer(modifier = Modifier.height(16.dp))

                Row(
                    horizontalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Button(
                        onClick = {
                            context.startActivity(
                                Intent(
                                    Intent.ACTION_VIEW,
                                    "https://github.com/aecsocket/wordbase/releases".toUri()
                                )
                            )
                        },
                    ) {
                        Text("Download latest version")
                    }

                    Button(
                        onClick = {
                            context.startActivity(
                                Intent(
                                    Intent.ACTION_VIEW,
                                    "https://github.com/aecsocket/wordbase/issues".toUri()
                                )
                            )
                        },
                    ) {
                        Text("Report bug")
                    }
                }
            }
        }
    }
}

@Composable
fun ReorderableCollectionItemScope.DictionaryRow(
    dictionary: Dictionary,
    profile: Profile,
    onSortingSet: () -> Unit,
    onSortingUnset: () -> Unit,
    onDelete: () -> Unit,
    onEnabledChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    ExpanderCard(
        modifier = modifier,
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
                    text = dictionary.meta.name, style = MaterialTheme.typography.bodyLarge
                )

                dictionary.meta.version?.let { version ->
                    Text(
                        text = version, style = MaterialTheme.typography.bodyMedium
                    )
                }
            }

            if (profile.sortingDictionary == dictionary.id) {
                IconButton(
                    onClick = onSortingUnset,
                    enabled = enabled,
                ) {
                    Icon(
                        painter = painterResource(R.drawable.outline_sort_24),
                        contentDescription = stringResource(R.string.dictionary_used_as_sorting)
                    )
                }
            }

            Switch(
                checked = profile.enabledDictionaries.contains(dictionary.id),
                onCheckedChange = onEnabledChange,
                enabled = enabled,
            )
        }
    ) {
        DictionaryRowColumn {
            DictionaryInfo(
                meta = dictionary.meta,
                onSetSorting = onSortingSet,
                onDelete = onDelete,
                enabled = enabled,
            )
        }
    }
}

sealed class DictionaryImport {
    abstract val id: Uuid
    abstract val error: String?

    abstract fun withError(error: String): DictionaryImport

    data class Started(
        override val id: Uuid,
        val fileName: String,
        override val error: String? = null,
    ) : DictionaryImport() {
        override fun withError(error: String) =
            Started(id, fileName, error)
    }

    data class ReadMeta(
        override val id: Uuid,
        val meta: DictionaryMeta,
        val progress: ImportProgress,
        override val error: String? = null,
    ) : DictionaryImport() {
        override fun withError(error: String) =
            ReadMeta(id, meta, progress, error)
    }
}

@Composable
fun ImportRow(import: DictionaryImport) {
    ExpanderCard(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = if (import.error == null) {
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
                val (progressAlpha, iconAlpha) = if (import.error == null) {
                    1f to 0f
                } else {
                    0f to 1f
                }

                when (import) {
                    is DictionaryImport.Started -> {
                        CircularProgressIndicator(
                            modifier = Modifier.alpha(progressAlpha)
                        )
                    }

                    is DictionaryImport.ReadMeta -> {
                        CircularProgressIndicator(
                            modifier = Modifier.alpha(progressAlpha),
                            progress = { import.progress.frac.toFloat() }
                        )
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

            val title = when (import) {
                is DictionaryImport.Started -> {
                    import.fileName
                }

                is DictionaryImport.ReadMeta -> {
                    import.meta.name
                }
            }
            val subtitle = if (import.error == null) {
                when (import) {
                    is DictionaryImport.Started -> null
                    is DictionaryImport.ReadMeta -> import.meta.version
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
        }
    ) {
        DictionaryRowColumn {
            import.error?.let { error ->
                DictionaryMetaItem(
                    key = stringResource(R.string.dictionary_import_error),
                    value = error
                )
            }

            when (import) {
                is DictionaryImport.Started -> {}
                is DictionaryImport.ReadMeta -> {
                    val fmt = NumberFormat.getPercentInstance()
                    fmt.minimumFractionDigits = 2
                    fmt.maximumFractionDigits = 2

                    DictionaryMetaItem(
                        key = stringResource(R.string.dictionary_import_progress),
                        value = fmt.format(import.progress.frac),
                    )
                }
            }

            when (import) {
                is DictionaryImport.Started -> {}
                is DictionaryImport.ReadMeta -> DictionaryInfo(meta = import.meta)
            }
        }
    }
}

@Composable
fun DictionaryRowColumn(
    content: @Composable ColumnScope.() -> Unit = {},
) {
    Column(
        modifier = Modifier.padding(8.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
        content = content
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ColumnScope.DictionaryInfo(
    meta: DictionaryMeta,
    onSetSorting: (() -> Unit)? = null,
    onDelete: (() -> Unit)? = null,
    enabled: Boolean = true,
) {
    DictionaryMetaItem(
        key = stringResource(R.string.dictionary_format), value = stringResource(
            when (meta.kind) {
                DictionaryKind.YOMITAN -> R.string.dictionary_format_yomitan
                DictionaryKind.YOMICHAN_AUDIO -> R.string.dictionary_format_yomichan_audio
            }
        )
    )

    meta.description?.let { description ->
        DictionaryMetaItem(
            key = stringResource(R.string.dictionary_description), value = description
        )
    }

    meta.attribution?.let { attribution ->
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

        onSetSorting?.let { onSetSortingDict ->
            IconButton(
                onClick = onSetSortingDict,
                enabled = enabled,
            ) {
                Icon(
                    painter = painterResource(R.drawable.outline_sort_24),
                    contentDescription = stringResource(R.string.dictionary_set_sorting)
                )
            }
        }

        meta.url?.let { url ->
            IconButton(
                onClick = {
                    val intent = Intent(Intent.ACTION_VIEW, url.toUri())
                    context.startActivity(intent)
                },
            ) {
                Icon(
                    painter = painterResource(R.drawable.outline_globe_24),
                    contentDescription = stringResource(R.string.dictionary_visit_website)
                )
            }
        }

        onDelete?.let { onDelete ->
            var openDeleteDialog by remember { mutableStateOf(false) }

            IconButton(
                onClick = {
                    openDeleteDialog = true
                },
                enabled = enabled,
            ) {
                Icon(
                    imageVector = Icons.Default.Delete,
                    contentDescription = stringResource(R.string.dictionary_delete)
                )
            }

            if (openDeleteDialog) {
                DictionaryDeleteDialog(
                    meta = meta,
                    onClose = {
                        openDeleteDialog = false
                    },
                    onConfirm = {
                        openDeleteDialog = false
                        onDelete()
                    }
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DictionaryDeleteDialog(meta: DictionaryMeta, onClose: () -> Unit, onConfirm: () -> Unit) {
    BasicAlertDialog(onDismissRequest = onClose) {
        Surface(
            modifier = Modifier.wrapContentSize(),
            shape = MaterialTheme.shapes.extraLarge,
            tonalElevation = AlertDialogDefaults.TonalElevation,
            color = MaterialTheme.colorScheme.surfaceContainerHigh
        ) {
            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                Text(
                    text = stringResource(R.string.dictionary_delete_title),
                    style = MaterialTheme.typography.labelLarge,
                )
                Text(
                    text = stringResource(R.string.dictionary_delete_body, meta.name)
                )

                Row(modifier = Modifier.align(Alignment.End)) {
                    TextButton(onClick = onClose) {
                        Text(text = stringResource(R.string.cancel))
                    }

                    TextButton(onClick = onConfirm) {
                        Text(text = stringResource(R.string.confirm))
                    }
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

//class ImportWorker(appContext: Context, workerParams: WorkerP)
