package io.github.aecsocket.wordbase

import android.content.ContentProvider
import android.content.ContentValues
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.net.toUri
import java.io.FileOutputStream

private const val TAG = "NoteProvider"

class NoteProvider : ContentProvider() {
    override fun onCreate() = true

    override fun query(
        uri: Uri,
        projection: Array<out String?>?,
        selection: String?,
        selectionArgs: Array<out String?>?,
        sortOrder: String?
    ) = null

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor? {
        Log.i(TAG, "Request to open note content $uri")
        val data = data ?: return null
        Log.i(TAG, "We have data, reading...")

        val (read, write) = ParcelFileDescriptor.createPipe()
        Thread {
            write.use { write ->
                FileOutputStream(write.fileDescriptor).use { stream ->
                    stream.write(data)
                }
            }
            Log.i(TAG, "All data written")
        }.start()
        Log.i(TAG, "Sent")
        return read
    }

    override fun getType(uri: Uri) = null

    override fun insert(
        uri: Uri,
        values: ContentValues?
    ) = null

    override fun delete(
        uri: Uri,
        selection: String?,
        selectionArgs: Array<out String?>?
    ) = 0

    override fun update(
        uri: Uri,
        values: ContentValues?,
        selection: String?,
        selectionArgs: Array<out String?>?
    ) = 0

    companion object {
        val uri = "content://io.github.aecsocket.wordbase.note".toUri()
        var data: ByteArray? = null
    }
}
