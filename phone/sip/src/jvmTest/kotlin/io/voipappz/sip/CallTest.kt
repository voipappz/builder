package io.voipappz.sip

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.random.Random
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/** One phone's WebSocket, wired to the in-memory switch. */
private class PipeTransport(private val toSwitch: suspend (SipMessage) -> Unit) : SipTransport {
    private val flow = MutableSharedFlow<SipMessage>(extraBufferCapacity = 64)
    override val incoming = flow
    override val closed = CompletableDeferred<Unit>()
    val sent = mutableListOf<SipMessage>()
    override suspend fun send(message: SipMessage) { sent += message; toSwitch(message) }
    override suspend fun close() { closed.complete(Unit) }
    suspend fun deliver(message: SipMessage) = flow.emit(message)
}

/**
 * A switch in memory that connects extensions: it challenges the caller's
 * INVITE (digest checked with its own MD5, not the production code), then
 * relays the call to the callee and every answer back, like a proxy.
 */
private class FakeCallSwitch(private val passwords: Map<String, String>) {
    private val phones = mutableMapOf<String, PipeTransport>()
    private var invite: SipRequest? = null
    private var caller: String? = null
    private var callee: String? = null
    val log = mutableListOf<String>()

    fun phone(extension: String) = PipeTransport { onMessage(extension, it) }.also { phones[extension] = it }

    private suspend fun onMessage(from: String, message: SipMessage) {
        when (message) {
            is SipResponse -> {
                log += "$from→ ${message.status}"
                phones[if (from == callee) caller!! else callee!!]!!.deliver(message)
            }
            is SipRequest -> {
                log += "$from→ ${message.method}"
                when (message.method) {
                    "INVITE" -> if (!authorised(message)) {
                        reply(from, message, 401, "Unauthorized",
                            "WWW-Authenticate" to "Digest realm=\"pbx.example.com\", nonce=\"n1\", qop=\"auth\"")
                    } else {
                        invite = message; caller = from
                        callee = message.uri.removePrefix("sip:").substringBefore('@')
                        phones[callee]!!.deliver(message)
                    }
                    // An ACK in the INVITE's own transaction (for a 401 or 487) ends here; the 2xx ACK goes on.
                    "ACK" -> if (invite != null && topBranch(message) != topBranch(invite!!)) phones[callee]!!.deliver(message)
                    "BYE" -> phones[if (from == caller) callee!! else caller!!]!!.deliver(message)
                    "CANCEL" -> {
                        reply(from, message, 200, "OK")
                        reply(from, invite!!, 487, "Request Terminated")
                    }
                }
            }
        }
    }

    private suspend fun reply(to: String, request: SipRequest, status: Int, reason: String, vararg extra: Pair<String, String>) {
        log += "switch→$to $status"
        phones[to]!!.deliver(SipResponse(status, reason,
            request.headers("Via").map { "Via" to it } + listOf(
                "To" to "${request.header("To")};tag=sw", "From" to request.header("From")!!,
                "Call-ID" to request.callId!!, "CSeq" to request.header("CSeq")!!) + extra))
    }

    private fun authorised(request: SipRequest): Boolean {
        val auth = request.header("Authorization") ?: return false
        val p = DigestChallenge.parseParams(auth.removePrefix("Digest"))
        val password = passwords[p["username"]] ?: return false
        val md5 = { t: String -> java.security.MessageDigest.getInstance("MD5").digest(t.toByteArray()).joinToString("") { "%02x".format(it) } }
        val ha1 = md5("${p["username"]}:${p["realm"]}:$password")
        val ha2 = md5("${request.method}:${p["uri"]}")
        return p["response"] == md5("$ha1:${p["nonce"]}:${p["nc"]}:${p["cnonce"]}:auth:$ha2")
    }
}

class CallTest {
    private val alice = SipAccount("1001", "secret", "pbx.example.com")

    @Test
    fun extension1001Calls1002WhoAnswersThen1001HangsUp() = runTest {
        val switch = FakeCallSwitch(mapOf("1001" to "secret"))
        val phoneA = switch.phone("1001")
        val phoneB = switch.phone("1002")

        // 1002: wait for the call, ring, answer, and confirm the hang-up.
        val receiver = async(start = CoroutineStart.UNDISPATCHED) {
            val invite = phoneB.incoming.first { it is SipRequest && it.method == "INVITE" } as SipRequest
            val call = IncomingCall(invite, phoneB, Random(2))
            val bye = async(start = CoroutineStart.UNDISPATCHED) { call.awaitBye() }
            call.ring()
            val acked = call.answer("v=0 (answer from 1002)")
            Triple(call.offer, acked, bye.await())
        }

        val call = OutgoingCall(alice, "1002", "v=0 (offer from 1001)", phoneA, Random(1))
        var rang = false
        val result = call.start { rang = true }

        assertEquals(CallResult.Answered("v=0 (answer from 1002)"), result)
        assertTrue(rang, "1001 heard it ringing")
        assertTrue(call.established)
        assertTrue(call.hangUp(), "1002 confirmed the hang-up")
        val (offerSeen, acked, byeSeen) = receiver.await()
        assertEquals("v=0 (offer from 1001)", offerSeen)
        assertTrue(acked, "1002 got the ACK")
        assertTrue(byeSeen, "1002 got the BYE")
        assertEquals(listOf(
            "1001→ INVITE", "switch→1001 401", "1001→ ACK",   // challenged, the 401 acknowledged
            "1001→ INVITE",                                     // again, with the digest answer
            "1002→ 180", "1002→ 200", "1001→ ACK",              // rings, answers, acknowledged
            "1001→ BYE", "1002→ 200",                           // hung up
        ), switch.log)

        // The in-dialog requests carry 1002's tag and go to its Contact.
        val bye = phoneA.sent.filterIsInstance<SipRequest>().last { it.method == "BYE" }
        assertTrue(tagOf(bye.header("To")) != null, bye.serialize())
        assertTrue(bye.uri.endsWith(".invalid;transport=ws"), bye.uri)
        val invites = phoneA.sent.filterIsInstance<SipRequest>().filter { it.method == "INVITE" }
        assertEquals(listOf(1, 2), invites.map { it.cseq!!.first })
        assertEquals(1, invites.map { it.callId }.distinct().size, "one Call-ID for the whole call")
    }

    @Test
    fun aRingingCallIsCancelled() = runTest {
        val switch = FakeCallSwitch(mapOf("1001" to "secret"))
        val phoneA = switch.phone("1001")
        val phoneB = switch.phone("1002")
        val receiver = async(start = CoroutineStart.UNDISPATCHED) {
            val invite = phoneB.incoming.first { it is SipRequest && it.method == "INVITE" } as SipRequest
            IncomingCall(invite, phoneB, Random(4)).ring() // rings, never answers
        }

        val call = OutgoingCall(alice, "1002", "v=0", phoneA, Random(3))
        val ringing = CompletableDeferred<Unit>()
        val result = async(start = CoroutineStart.UNDISPATCHED) { call.start { ringing.complete(Unit) } }
        ringing.await()
        assertTrue(call.cancel())
        assertEquals(CallResult.Failed(487, "Request Terminated"), result.await())
        assertFalse(call.established)
        receiver.await()
        // CANCEL uses the INVITE's branch and CSeq number; the 487 is acknowledged.
        val sent = phoneA.sent.filterIsInstance<SipRequest>()
        val invite = sent.last { it.method == "INVITE" }
        val cancel = sent.single { it.method == "CANCEL" }
        assertEquals(topBranch(invite), topBranch(cancel))
        assertEquals(invite.cseq!!.first, cancel.cseq!!.first)
        assertEquals("ACK", sent.last().method)
    }

    @Test
    fun aWrongPasswordFailsTheCallWithoutLooping() = runTest {
        val switch = FakeCallSwitch(mapOf("1001" to "other"))
        val phoneA = switch.phone("1001")
        switch.phone("1002")
        val result = OutgoingCall(alice, "1002", "v=0", phoneA, Random(5)).start()
        assertEquals(CallResult.Failed(401, "Unauthorized"), result)
        assertEquals(2, phoneA.sent.count { it is SipRequest && it.method == "INVITE" })
    }
}
