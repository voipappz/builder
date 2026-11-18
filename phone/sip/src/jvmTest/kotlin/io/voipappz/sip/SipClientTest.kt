package io.voipappz.sip

import kotlin.random.Random
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

// Desktop-JVM only: the in-memory switch checks the digest with
// java.security.MessageDigest, independent of the production code.

/**
 * A switch in memory. It checks the digest with its own MD5 arithmetic (not
 * the production [Digest]), so a bug there cannot hide itself. [nonces]
 * hands out a new nonce per challenge when set, to test the retry bound.
 */
private class FakeSwitch(
    private val password: String,
    private val minExpires: Int? = null,
    private val rotateNonce: Boolean = false,
) : SipTransport {
    private val out = kotlinx.coroutines.flow.MutableSharedFlow<SipMessage>(extraBufferCapacity = 16)
    override val incoming = out
    val ended = kotlinx.coroutines.CompletableDeferred<Unit>()
    override val closed = ended
    val received = mutableListOf<SipRequest>()
    private var nonce = 0

    override suspend fun send(message: SipMessage) {
        val request = message as SipRequest
        received += request
        val echo = listOf("Via" to request.header("Via")!!, "Call-ID" to request.callId!!, "CSeq" to request.header("CSeq")!!)
        val auth = request.header("Authorization")
        val challenge = { "Digest realm=\"pbx.example.com\", nonce=\"n${if (rotateNonce) ++nonce else 1}\", qop=\"auth\"" }
        val reply = when {
            auth == null -> SipResponse(401, "Unauthorized", echo + ("WWW-Authenticate" to challenge()))
            !rightDigest(request, auth) -> SipResponse(401, "Unauthorized", echo + ("WWW-Authenticate" to challenge()))
            minExpires != null && request.header("Expires")!!.toInt() in 1 until minExpires ->
                SipResponse(423, "Interval Too Brief", echo + ("Min-Expires" to "$minExpires"))
            else -> SipResponse(200, "OK", echo + ("Contact" to "${request.header("Contact")!!.substringBefore(";expires")};expires=${request.header("Expires")}"))
        }
        out.emit(SipResponse(100, "Trying", echo))
        out.emit(reply)
    }

    private fun rightDigest(request: SipRequest, auth: String): Boolean {
        val p = DigestChallenge.parseParams(auth.removePrefix("Digest"))
        val md5 = { t: String -> java.security.MessageDigest.getInstance("MD5").digest(t.toByteArray()).joinToString("") { "%02x".format(it) } }
        val ha1 = md5("${p["username"]}:${p["realm"]}:$password")
        val ha2 = md5("${request.method}:${p["uri"]}")
        return p["response"] == md5("$ha1:${p["nonce"]}:${p["nc"]}:${p["cnonce"]}:auth:$ha2")
    }

    override suspend fun close() { ended.complete(Unit) }
}

class SipClientTest {
    private val account = SipAccount(username = "1001", password = "secret", domain = "pbx.example.com")

    @Test
    fun registersAfterOneChallenge() = kotlinx.coroutines.test.runTest {
        val switch = FakeSwitch(password = "secret")
        val result = SipClient(switch).register(Registration(account, random = Random(5)))
        assertEquals(RegisterResult.Registered(600), result)
        assertEquals(2, switch.received.size)
    }

    @Test
    fun aWrongPasswordIsRefusedWithoutLooping() = kotlinx.coroutines.test.runTest {
        val switch = FakeSwitch(password = "other")
        val result = SipClient(switch).register(Registration(account, random = Random(6)))
        assertEquals(RegisterResult.Refused(401, "Unauthorized"), result)
        assertEquals(2, switch.received.size)
    }

    @Test
    fun aSwitchThatKeepsChangingTheNonceIsAnsweredAtMostTwice() = kotlinx.coroutines.test.runTest {
        val switch = FakeSwitch(password = "other", rotateNonce = true)
        val result = SipClient(switch).register(Registration(account, random = Random(7)))
        assertEquals(RegisterResult.Refused(401, "Unauthorized"), result)
        assertEquals(1 + Registration.MAX_ANSWERS, switch.received.size)
    }

    @Test
    fun aTooShortIntervalIsRaisedToTheSwitchsMinimum() = kotlinx.coroutines.test.runTest {
        val switch = FakeSwitch(password = "secret", minExpires = 900)
        val result = SipClient(switch).register(Registration(account, expires = 300, random = Random(8)))
        assertEquals(RegisterResult.Registered(900), result)
    }

    @Test
    fun aClosedConnectionFailsTheRequestAtOnce() = kotlinx.coroutines.test.runTest {
        val silent = object : SipTransport {
            override val incoming = kotlinx.coroutines.flow.MutableSharedFlow<SipMessage>()
            val ended = kotlinx.coroutines.CompletableDeferred<Unit>()
            override val closed = ended
            override suspend fun send(message: SipMessage) { ended.complete(Unit) }
            override suspend fun close() {}
        }
        val error = runCatching { SipClient(silent).register(Registration(account, random = Random(9))) }.exceptionOrNull()
        assertIs<TransportClosedException>(error)
    }
}

