package io.voipappz.sip

import kotlin.random.Random
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SipCoreTest {
    // Fake values only (never a real extension or password).
    private val account = SipAccount(username = "1001", password = "secret", domain = "pbx.example.com")

    @Test
    fun digestMatchesTheRfc2617Example() {
        // RFC 2617 section 3.5.
        val challenge = DigestChallenge(realm = "testrealm@host.com", nonce = "dcd98b7102dd2f0e8b11d0f600bfb0c093",
            opaque = "5ccc069c403ebaf9f0171e9517f40e41", qop = listOf("auth", "auth-int"))
        val value = Digest.authorization(challenge, "GET", "/dir/index.html", "Mufasa", "Circle Of Life",
            cnonce = "0a4f113b", nc = 1)
        assertTrue(value.contains("response=\"6629fae49393a05397450978507c4ef1\""), value)
        assertTrue(value.contains("qop=auth, nc=00000001, cnonce=\"0a4f113b\""), value)
        assertTrue(value.contains("opaque=\"5ccc069c403ebaf9f0171e9517f40e41\""), value)
    }

    @Test
    fun aChallengeIsRead() {
        val c = assertNotNull(DigestChallenge.parse(
            "Digest realm=\"pbx.example.com\", nonce=\"abc,def\", algorithm=MD5, qop=\"auth\""))
        assertEquals("pbx.example.com", c.realm)
        assertEquals("abc,def", c.nonce)
        assertEquals(listOf("auth"), c.qop)
        assertNull(DigestChallenge.parse("Basic realm=\"x\""))
    }

    @Test
    fun aResponseIsParsedWithRepeatedAndCompactHeaders() {
        val text = "SIP/2.0 401 Unauthorized\r\n" +
            "v: SIP/2.0/WSS a.invalid;branch=z9hG4bK1\r\n" +
            "Via: SIP/2.0/WSS b.invalid;branch=z9hG4bK2\r\n" +
            "i: call-1\r\n" +
            "CSeq: 1 REGISTER\r\n" +
            "WWW-Authenticate: Digest realm=\"pbx.example.com\",\r\n" +
            "  nonce=\"n1\", qop=\"auth\"\r\n" +
            "Content-Length: 0\r\n\r\n"
        val response = assertIs<SipResponse>(SipMessage.parse(text))
        assertEquals(401, response.status)
        assertEquals(2, response.headers("Via").size)
        assertEquals("call-1", response.callId)
        assertEquals(1 to "REGISTER", response.cseq)
        assertEquals("n1", DigestChallenge.parse(response.header("WWW-Authenticate")!!)?.nonce)
    }

    @Test
    fun aMessageSurvivesSerializeAndParse() {
        val request = Registration(account, random = Random(1)).register()
        val again = assertIs<SipRequest>(SipMessage.parse(request.serialize()))
        assertEquals("REGISTER", again.method)
        assertEquals("sip:pbx.example.com", again.uri)
        assertEquals(request.headers, again.headers.filterNot { it.first == "Content-Length" })
        assertTrue(request.serialize().endsWith("Content-Length: 0\r\n\r\n"))
    }

    @Test
    fun theRegisterIsShapedForAWebSocket() {
        val registration = Registration(account, random = Random(2))
        val request = registration.register()
        assertTrue(request.header("Via")!!.startsWith("SIP/2.0/WSS ${registration.viaHost};branch=z9hG4bK"))
        assertTrue(registration.viaHost.endsWith(".invalid"))
        assertEquals("<sip:1001@pbx.example.com>", request.header("To"))
        assertTrue(request.header("Contact")!!.contains(";transport=ws>;expires=600"))
        assertEquals(1 to "REGISTER", request.cseq)
    }

    @Test
    fun a401IsAnsweredOnceAndAWrongPasswordDoesNotLoop() {
        val registration = Registration(account, random = Random(3))
        val first = registration.register()
        val challenge = SipResponse(401, "Unauthorized", listOf(
            "Call-ID" to first.callId!!, "CSeq" to "1 REGISTER",
            "WWW-Authenticate" to "Digest realm=\"pbx.example.com\", nonce=\"n1\", qop=\"auth\""))

        val second = assertNotNull(registration.authenticate(first, challenge))
        assertEquals(2 to "REGISTER", second.cseq)
        assertEquals(first.callId, second.callId)
        assertTrue(second.header("Authorization")!!.contains("username=\"1001\""))
        assertTrue(second.header("Authorization")!!.contains("uri=\"sip:pbx.example.com\""))
        assertTrue(second.header("Via") != first.header("Via"), "a new branch for the new transaction")

        // The switch refuses the answer with the same nonce: the password is wrong; stop.
        assertNull(registration.authenticate(second, challenge))
        // A stale nonce is answered again with the new one.
        val stale = SipResponse(401, "Unauthorized", listOf(
            "WWW-Authenticate" to "Digest realm=\"pbx.example.com\", nonce=\"n2\", qop=\"auth\", stale=true"))
        assertNotNull(registration.authenticate(second, stale))
    }

    @Test
    fun theGrantedTimeIsReadFromTheSwitchsAnswer() {
        val registration = Registration(account, random = Random(4))
        val ok = SipResponse(200, "OK", listOf(
            "Contact" to "<sip:other@x.invalid;transport=ws>;expires=900",
            "Contact" to "${registration.contact};expires=300"))
        assertEquals(300, registration.grantedExpires(ok))
        assertEquals(600, registration.grantedExpires(SipResponse(200, "OK", emptyList())))
    }
}

/** A switch in memory: challenges the first REGISTER, accepts the right digest, refuses a wrong one. */
private class FakeSwitch(private val password: String) : SipTransport {
    private val out = kotlinx.coroutines.flow.MutableSharedFlow<SipMessage>(extraBufferCapacity = 16)
    override val incoming = out
    val received = mutableListOf<SipRequest>()

    override suspend fun send(message: SipMessage) {
        val request = message as SipRequest
        received += request
        val echo = listOf("Call-ID" to request.callId!!, "CSeq" to request.header("CSeq")!!)
        val auth = request.header("Authorization")
        val reply = if (auth == null) {
            SipResponse(401, "Unauthorized", echo + ("WWW-Authenticate" to "Digest realm=\"pbx.example.com\", nonce=\"n1\", qop=\"auth\""))
        } else {
            val p = DigestChallenge.parseParams(auth.removePrefix("Digest"))
            val expected = Digest.authorization(DigestChallenge("pbx.example.com", "n1", qop = listOf("auth")),
                request.method, request.uri, p["username"]!!, password, p["cnonce"]!!, p["nc"]!!.toInt(16))
            if (expected == auth) SipResponse(100, "Trying", echo).also { out.emit(it) }
                .let { SipResponse(200, "OK", echo + ("Contact" to request.header("Contact")!!)) }
            else SipResponse(401, "Unauthorized", echo + ("WWW-Authenticate" to "Digest realm=\"pbx.example.com\", nonce=\"n1\", qop=\"auth\""))
        }
        out.emit(reply)
    }

    override suspend fun close() {}
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
}
