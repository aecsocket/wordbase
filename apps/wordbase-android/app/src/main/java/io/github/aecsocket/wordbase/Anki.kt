package io.github.aecsocket.wordbase

import android.content.Context
import androidx.compose.runtime.Composable
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.PermissionState
import com.google.accompanist.permissions.PermissionStatus
import com.google.accompanist.permissions.rememberPermissionState
import com.ichi2.anki.api.AddContentApi

data class Anki(
    private val context: Context,
    val packageName: String,
) {
    @OptIn(ExperimentalPermissionsApi::class)
    fun api(permission: PermissionStatus.Granted): AddContentApi {
        return AddContentApi(context)
    }
}

fun Context.anki(): Anki? {
    val packageName = AddContentApi.getAnkiDroidPackageName(this) ?: return null
    return Anki(context = this, packageName = packageName)
}

@OptIn(ExperimentalPermissionsApi::class)
@Composable
fun rememberAnkiPermissionState(): PermissionState {
    return rememberPermissionState(AddContentApi.READ_WRITE_PERMISSION)
}
