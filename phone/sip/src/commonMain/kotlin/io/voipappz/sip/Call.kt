package io.voipappz.sip

import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.selects.select
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeout
import kotlin.random.Random

/** Random identifiers for tags, Call-IDs and branches. */
internal class SipIds(private val random: Random) {
    fun token(length: Int): String {
        val chars = "abcdefghijklmnopqrstuvwxyz0123456789"
        return (1..length).map { chars[random.nextInt(chars.length)] }.joinToString("")
    }
    fun branch() = "z9hG4bK" + token(16)
}

/** The top Via's branch: the transaction a message belongs to. */
internal fun topBranch(message: SipMessage): String? =
    message.header("Via")?.let { Regex("branch=([^;,\\s]+)", RegexOption.IGNORE_CASE).find(it)?.groupValues?.get(1) }

/** The tag parameter of a From/To value. */
internal fun tagOf(value: String?): String? =
    value?.let { Regex(";\\s*tag=([^;>\\s]+)", RegexOption.IGNORE_CASE).find(it)?.groupValues?.get(1) }

/** The URI inside `<...>` (or the whole value without parameters). */
internal fun uriOf(value: String): String =
    if ('<' in value) value.substringAfter('<').substringBefore('>') else value.substringBefore(';').trim()

/** How an outgoing call ended up. */
sealed class CallResult {
    /** Answered: [answer] is the far end's SDP. */
    data class Answered(val answer: String) : CallResult()
    data class Failed(val status: Int, val reason: String) : CallResult()
}

/**
 * One outgoing call (RFC 3261 UAC, over the WebSocket): INVITE, a digest
 * challenge answered once, provisional answers (180/183 ring), the 2xx
 * acknowledged with ACK, then BYE to hang up or CANCEL while it rings.
 * The SDP offer comes from the media layer (webrtc-kmp) and is sent as is.
 */
class OutgoingCall(
    private val account: SipAccount,
    /** The number or extension to call, e.g. `1002`. */
    target: String,
    private val offer: String,
    private val transport: SipTransport,
    random: Random = Random.Default,
    private val timeoutMillis: Long = 60_000,
    private val userAgent: String = "VoipAppz Phone",
) {
    private val ids = SipIds(random)
    private val viaHost = "${ids.token(12)}.invalid"
    private val contact = "<sip:${ids.token(8)}@$viaHost;transport=ws>"
    private val targetUri = if (target.startsWith("sip:")) target else "sip:$target@${account.domain}"
    val callId: String = ids.token(22)
    private val fromTag = ids.token(10)
    private var cseq = 0
    private var nonceCount = 0
    private var lastNonce: String? = null
    private var invite: SipRequest? = null
    private var toTag: String? = null
    private var remoteTarget: String = targetUri
    private var routeSet: List<String> = emptyList()
    private val lock = Mutex()

    /** True between a 2xx and the hang-up. */
    var established = false
        private set

    /**
     * Places the call and waits for its final answer. [onRinging] runs on
     * each 180/183. A non-2xx final answer is acknowledged and returned.
     */
    suspend fun start(onRinging: (status: Int) -> Unit = {}): CallResult {
        var request = newInvite(authorization = null)
        var answeredChallenge = false
        while (true) {
            invite = request
            val response = finalResponse(request, onRinging)
            when {
                response.status in 200..299 -> {
                    toTag = tagOf(response.header("To"))
                    response.header("Contact")?.let { remoteTarget = uriOf(it) }
                    routeSet = response.headers("Record-Route").flatMap(::splitContacts).reversed()
                    transport.send(ackFor2xx(response))
                    established = true
                    return CallResult.Answered(response.body)
                }
                (response.status == 401 || response.status == 407) && !answeredChallenge -> {
                    transport.send(ackForNon2xx(request, response))
                    val proxy = response.status == 407
                    val challenge = response.header(if (proxy) "Proxy-Authenticate" else "WWW-Authenticate")
                        ?.let(DigestChallenge::parse) ?: return CallResult.Failed(response.status, response.reason)
                    val digest = answer(challenge, "INVITE", targetUri)
                        ?: return CallResult.Failed(response.status, response.reason)
                    val header = (if (proxy) "Proxy-Authorization" else "Authorization") to digest
                    answeredChallenge = true
                    request = newInvite(authorization = header)
                }
                else -> {
                    transport.send(ackForNon2xx(request, response))
                    return CallResult.Failed(response.status, response.reason)
                }
            }
        }
    }

    /** Hangs up an answered call (BYE); true when the far end confirmed. */
    suspend fun hangUp(): Boolean = lock.withLock {
        if (!established) return false
        established = false
        cseq++
        val bye = inDialog("BYE", cseq)
        exchange(bye).status in 200..299
    }

    /**
     * Cancels the call while it rings (CANCEL, same branch and CSeq number
     * as the INVITE). The INVITE then ends with 487, which [start] returns.
     */
    suspend fun cancel(): Boolean {
        val inv = invite ?: return false
        if (established) return false
        val number = inv.cseq?.first ?: return false
        val cancel = SipRequest("CANCEL", inv.uri, listOf(
            "Via" to inv.header("Via")!!,
            "Max-Forwards" to "70",
            "To" to inv.header("To")!!,
            "From" to inv.header("From")!!,
            "Call-ID" to callId,
            "CSeq" to "$number CANCEL",
            "User-Agent" to userAgent,
        ))
        return exchange(cancel).status in 200..299
    }

    private fun newInvite(authorization: Pair<String, String>?): SipRequest {
        cseq++
        val headers = mutableListOf(
            "Via" to "SIP/2.0/WSS $viaHost;branch=${ids.branch()}",
            "Max-Forwards" to "70",
            "To" to "<$targetUri>",
            "From" to "<sip:${account.username}@${account.domain}>;tag=$fromTag",
            "Call-ID" to callId,
            "CSeq" to "$cseq INVITE",
            "Contact" to contact,
            "Allow" to "INVITE,ACK,CANCEL,BYE,OPTIONS,INFO,NOTIFY,REFER",
            "Supported" to "outbound",
            "User-Agent" to userAgent,
            "Content-Type" to "application/sdp",
        )
        authorization?.let { headers += it }
        return SipRequest("INVITE", targetUri, headers, offer)
    }

    private fun answer(challenge: DigestChallenge, method: String, uri: String): String? {
        if (challenge.qop.isNotEmpty() && challenge.qop.none { it.equals("auth", ignoreCase = true) }) return null
        nonceCount = if (challenge.nonce == lastNonce) nonceCount + 1 else 1
        lastNonce = challenge.nonce
        return Digest.authorization(challenge, method, uri, account.username, account.password,
            cnonce = ids.token(16), nc = nonceCount)
    }

    /** ACK for a non-2xx final answer: same transaction (branch), CSeq number, To with its tag (RFC 3261 17.1.1.3). */
    private fun ackForNon2xx(invite: SipRequest, response: SipResponse) = SipRequest("ACK", invite.uri, listOf(
        "Via" to invite.header("Via")!!,
        "Max-Forwards" to "70",
        "To" to (response.header("To") ?: invite.header("To")!!),
        "From" to invite.header("From")!!,
        "Call-ID" to callId,
        "CSeq" to "${invite.cseq!!.first} ACK",
        "User-Agent" to userAgent,
    ))

    /** ACK for a 2xx: its own transaction, to the remote target, through the route set (RFC 3261 13.2.2.4). */
    private fun ackFor2xx(response: SipResponse) = inDialog("ACK", response.cseq!!.first)

    /** A request inside the dialog: remote target as Request-URI, both tags, the route set. */
    private fun inDialog(method: String, number: Int): SipRequest {
        val headers = mutableListOf(
            "Via" to "SIP/2.0/WSS $viaHost;branch=${ids.branch()}",
            "Max-Forwards" to "70",
        )
        routeSet.forEach { headers += "Route" to it }
        headers += listOf(
            "To" to "<$targetUri>" + (toTag?.let { ";tag=$it" } ?: ""),
            "From" to "<sip:${account.username}@${account.domain}>;tag=$fromTag",
            "Call-ID" to callId,
            "CSeq" to "$number $method",
            "User-Agent" to userAgent,
        )
        return SipRequest(method, remoteTarget, headers)
    }

    /** The INVITE's final response; provisional ones go to [onRinging]. */
    private suspend fun finalResponse(request: SipRequest, onRinging: (Int) -> Unit): SipResponse =
        withTimeout(timeoutMillis) {
            coroutineScope {
                val branch = topBranch(request)
                val final = async(start = CoroutineStart.UNDISPATCHED) {
                    transport.incoming.first {
                        if (it !is SipResponse || it.callId != callId || it.cseq != request.cseq || topBranch(it) != branch) {
                            false
                        } else if (it.status < 200) {
                            if (it.status == 180 || it.status == 183) onRinging(it.status)
                            false
                        } else true
                    } as SipResponse
                }
                transport.send(request)
                select {
                    final.onAwait { it }
                    transport.closed.onAwait { final.cancel(); throw TransportClosedException() }
                }
            }
        }

    /** A non-INVITE transaction (BYE, CANCEL): its final response. */
    private suspend fun exchange(request: SipRequest): SipResponse = withTimeout(32_000) {
        coroutineScope {
            val branch = topBranch(request)
            val answer = async(start = CoroutineStart.UNDISPATCHED) {
                transport.incoming.first {
                    it is SipResponse && it.status >= 200 && it.callId == callId && it.cseq == request.cseq &&
                        topBranch(it) == branch
                } as SipResponse
            }
            transport.send(request)
            select {
                answer.onAwait { it }
                transport.closed.onAwait { answer.cancel(); throw TransportClosedException() }
            }
        }
    }
}

/**
 * One incoming call (RFC 3261 UAS): built from the INVITE the switch sent.
 * Ring with [ring], then [answer] with the SDP answer from the media layer
 * (or [reject]); [awaitBye] answers the far end's hang-up.
 */
class IncomingCall(
    val invite: SipRequest,
    private val transport: SipTransport,
    random: Random = Random.Default,
    private val userAgent: String = "VoipAppz Phone",
) {
    private val ids = SipIds(random)
    private val localTag = ids.token(10)
    private val contact = "<sip:${ids.token(8)}@${ids.token(12)}.invalid;transport=ws>"
    val callId: String = invite.callId ?: error("INVITE without Call-ID")
    /** The far end's SDP offer. */
    val offer: String get() = invite.body
    /** Who calls: the From URI. */
    val caller: String get() = uriOf(invite.header("From") ?: "")

    suspend fun ring() = transport.send(response(180, "Ringing"))

    /** Answers with [answer] (SDP) and waits for the ACK. */
    suspend fun answer(answer: String, timeoutMillis: Long = 32_000): Boolean = coroutineScope {
        val ack = async(start = CoroutineStart.UNDISPATCHED) {
            transport.incoming.first { it is SipRequest && it.method == "ACK" && it.callId == callId }
        }
        transport.send(response(200, "OK", body = answer, extra = listOf("Contact" to contact, "Content-Type" to "application/sdp")))
        runCatching { withTimeout(timeoutMillis) { ack.await() } }.isSuccess
    }

    suspend fun reject(status: Int = 486, reason: String = "Busy Here") = transport.send(response(status, reason))

    /** Waits for the far end's BYE and confirms it with 200. */
    suspend fun awaitBye(timeoutMillis: Long = 3_600_000): Boolean {
        val bye = runCatching {
            withTimeout(timeoutMillis) {
                transport.incoming.first { it is SipRequest && it.method == "BYE" && it.callId == callId } as SipRequest
            }
        }.getOrNull() ?: return false
        transport.send(SipResponse(200, "OK", listOf(
            "Via" to bye.header("Via")!!, "To" to bye.header("To")!!, "From" to bye.header("From")!!,
            "Call-ID" to callId, "CSeq" to bye.header("CSeq")!!, "User-Agent" to userAgent)))
        return true
    }

    /** A response to the INVITE: its Vias, From, Call-ID and CSeq, our To tag. */
    private fun response(status: Int, reason: String, body: String = "", extra: List<Pair<String, String>> = emptyList()): SipResponse {
        val to = invite.header("To")!!.let { if (tagOf(it) == null) "$it;tag=$localTag" else it }
        return SipResponse(status, reason,
            invite.headers("Via").map { "Via" to it } + listOf(
                "To" to to, "From" to invite.header("From")!!, "Call-ID" to callId,
                "CSeq" to invite.header("CSeq")!!, "User-Agent" to userAgent) + extra,
            body)
    }
}
