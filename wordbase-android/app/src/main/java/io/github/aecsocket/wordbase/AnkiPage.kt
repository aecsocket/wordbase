package io.github.aecsocket.wordbase

import android.content.Intent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
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
import androidx.core.net.toUri
import kotlinx.coroutines.launch

@Preview(showBackground = true)
@Composable
fun AnkiPagePreview() {
    val decks = listOf(
        "Kaishi 1.5k",
        "Mining",
        "Testing",
    )
    var deck by remember { mutableStateOf(decks[0]) }

    val models = listOf(
        "Lapis",
        "Kaishi 1.5k",
    )
    var model by remember { mutableStateOf(models[0]) }

    AnkiPage(
        deck = deck,
        decks = decks,
        onDeckChange = { deck = it },
        model = model,
        models = models,
        onModelChange = { model = it },
        enabled = true,
    )
}

@Composable
fun AnkiPageApp(
    enabled: Boolean,
) {
    val context = LocalContext.current
    val app = context.app()
    val wordbase by rememberWordbase()
    val coroutineScope = rememberCoroutineScope()

    context.anki()?.let { anki ->
        val profile = app.profiles[app.profileId]

        AnkiPage(
            deck = profile?.ankiDeck ?: "",
            decks = anki.deckList.values.toList(),
            onDeckChange = { deck ->
                val wordbase = wordbase ?: return@AnkiPage
                coroutineScope.launch {
                    app.writeToWordbase(wordbase) {
                        wordbase.setAnkiDeck(app.profileId, deck)
                    }
                }
            },
            model = profile?.ankiNoteType ?: "",
            models = anki.modelList.values.toList(),
            onModelChange = { model ->
                val wordbase = wordbase ?: return@AnkiPage
                coroutineScope.launch {
                    app.writeToWordbase(wordbase) {
                        wordbase.setAnkiNoteType(app.profileId, model)
                    }
                }
            },
            enabled = enabled,
        )
    } ?: run {
        Column(
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                text = stringResource(R.string.add_note_no_anki)
            )

            val context = LocalContext.current
            Button(
                modifier = Modifier.fillMaxWidth(),
                onClick = {
                    context.startActivity(
                        Intent(
                            Intent.ACTION_VIEW,
                            "market://details?id=com.ichi2.anki".toUri()
                        )
                    )
                },
            ) {
                Text(
                    text = stringResource(R.string.manage_anki_install)
                )
            }
        }
    }
}

@Composable
fun AnkiPage(
    deck: String,
    decks: List<String>,
    onDeckChange: (String) -> Unit,
    model: String,
    models: List<String>,
    onModelChange: (String) -> Unit,
    enabled: Boolean,
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        DropdownField(
            label = {
                Text(
                    text = stringResource(R.string.manage_anki_deck),
                )
            },
            selected = deck,
            options = decks,
            onSelectedChange = onDeckChange,
            enabled = enabled,
        )

        DropdownField(
            label = {
                Text(
                    text = stringResource(R.string.manage_anki_note_type),
                )
            },
            selected = model,
            options = models,
            onSelectedChange = onModelChange,
            enabled = enabled,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DropdownField(
    label: @Composable () -> Unit,
    selected: String,
    options: List<String>,
    onSelectedChange: (String) -> Unit,
    enabled: Boolean = true,
) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
    ) {
        TextField(
            modifier = Modifier
                .menuAnchor(MenuAnchorType.PrimaryNotEditable)
                .fillMaxWidth(),
            value = selected,
            onValueChange = {},
            readOnly = true,
            singleLine = true,
            label = label,
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            colors = ExposedDropdownMenuDefaults.textFieldColors(),
            enabled = enabled,
        )

        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false }
        ) {
            options.forEach {
                DropdownMenuItem(
                    enabled = enabled,
                    text = { Text(text = it) },
                    onClick = {
                        onSelectedChange(it)
                        expanded = false
                    },
                    contentPadding = ExposedDropdownMenuDefaults.ItemContentPadding,
                )
            }
        }
    }
}
