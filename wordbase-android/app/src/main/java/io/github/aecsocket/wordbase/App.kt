package io.github.aecsocket.wordbase

import android.app.Application
import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import uniffi.wordbase.DictionaryEvent
import uniffi.wordbase.EngineEvent
import uniffi.wordbase.Wordbase
import uniffi.wordbase.wordbase
import uniffi.wordbase_api.Dictionary
import uniffi.wordbase_api.DictionaryId
import uniffi.wordbase_api.Profile
import uniffi.wordbase_api.ProfileId
import java.io.File

class App : Application() {
    lateinit var wordbase: Deferred<Wordbase>

    private var _dictionaries by mutableStateOf(mapOf<DictionaryId, Dictionary>())
    val dictionaries get() = _dictionaries

    private var _profiles by mutableStateOf(mapOf<ProfileId, Profile>())
    val profiles get() = _profiles

    override fun onCreate() {
        super.onCreate()

        // todo
//        File(filesDir, "wordbase.db-shm").delete()
//        File(filesDir, "wordbase.db-wal").delete()
//        val file = File(filesDir, "wordbase.db")
//        file.delete()
//        assets.open("wordbase.db").use { input ->
//            file.outputStream().use { output ->
//                input.copyTo(output)
//            }
//        }

        val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

        wordbase = appScope.async {
            val engine = wordbase(filesDir.absolutePath)
            _dictionaries = engine.dictionaries()
            _profiles = engine.profiles()

            appScope.launch {
                val eventRx = engine.eventRx()
                while (true) {
                    val event = eventRx.recv() ?: return@launch
                    onWordbaseEvent(engine, event)
                }
            }

            engine
        }
    }

    private fun onWordbaseEvent(wordbase: Wordbase, event: EngineEvent) {
        // TODO clean this up on rust side as well
        when (event) {
            is EngineEvent.TexthookerConnected -> {}
            is EngineEvent.TexthookerDisconnected -> {}
            is EngineEvent.Sentence -> {}
            is EngineEvent.Dictionary -> {
                if (event.v1 !is DictionaryEvent.PositionsSwapped) {
                    _dictionaries = wordbase.dictionaries()
                    _profiles = wordbase.profiles()
                }
            }
            else -> {
                _dictionaries = wordbase.dictionaries()
                _profiles = wordbase.profiles()
            }
        }
    }

    suspend fun swapDictionaryPositions(aId: DictionaryId, bId: DictionaryId) {
        val wordbase = wordbase.await()
        wordbase.swapDictionaryPositions(aId, bId)
        _dictionaries = wordbase.dictionaries()
    }
}

fun Context.app() = applicationContext as App

@Composable fun rememberWordbase(): MutableState<Wordbase?> {
    val state = remember { mutableStateOf<Wordbase?>(null) }
    val context = LocalContext.current
    LaunchedEffect(Unit) {
        state.value = context.app().wordbase.await()
    }
    return state
}
