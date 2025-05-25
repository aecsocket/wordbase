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
import uniffi.wordbase.Wordbase
import uniffi.wordbase.wordbase
import uniffi.wordbase_api.Dictionary
import uniffi.wordbase_api.DictionaryId
import uniffi.wordbase_api.Profile
import uniffi.wordbase_api.ProfileId

class App : Application() {
    lateinit var wordbase: Deferred<Wordbase>

    private var _dictionaries by mutableStateOf(mapOf<DictionaryId, Dictionary>())
    val dictionaries get() = _dictionaries

    private var _profiles by mutableStateOf(mapOf<ProfileId, Profile>())
    val profiles get() = _profiles

    override fun onCreate() {
        super.onCreate()
        val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

        wordbase = appScope.async {
            val engine = wordbase(filesDir.absolutePath)
            _dictionaries = engine.dictionaries()
            _profiles = engine.profiles()
            engine
        }
    }

    suspend fun <R> writeToWordbase(wordbase: Wordbase, block: suspend () -> R): R {
        val r = block()
        _dictionaries = wordbase.dictionaries()
        _profiles = wordbase.profiles()
        return r
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
