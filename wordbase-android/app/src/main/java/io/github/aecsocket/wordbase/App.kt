package io.github.aecsocket.wordbase

import android.app.Application
import android.content.Context
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import uniffi.wordbase.Engine
import java.io.File

class App : Application() {
    lateinit var engine: Deferred<Engine>

    override fun onCreate() {
        super.onCreate()

        File(filesDir, "wordbase.db-shm").delete()
        File(filesDir, "wordbase.db-wal").delete()
        val file = File(filesDir, "wordbase.db")
        file.delete()
        assets.open("wordbase.db").use { input ->
            file.outputStream().use { output ->
                input.copyTo(output)
            }
        }

        engine = CoroutineScope(SupervisorJob() + Dispatchers.Default).async {
            uniffi.wordbase.engine(filesDir.absolutePath)
        }
    }
}

suspend fun Context.wordbase(): Engine = (applicationContext as App).engine.await()
