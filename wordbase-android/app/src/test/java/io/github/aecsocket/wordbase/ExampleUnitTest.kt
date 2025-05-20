package io.github.aecsocket.wordbase

import com.sun.jna.Native
import org.junit.Test

import org.junit.Assert.*

/**
 * Example local unit test, which will execute on the development machine (host).
 *
 * See [testing documentation](http://d.android.com/tools/testing).
 */
class ExampleUnitTest {
    @Test
    fun addition_isCorrect() {
        assertEquals(5, uniffi.wordbase.add(2u, 3u))
    }
}
