package io.voipappz.sip

import kotlin.random.Random

/** One SIP account: `username@domain` with its password. */
data class SipAccount(
    val username: String,
    val password: String,
    /** The SIP domain, e.g. `switch.voipappz.io`. */
    val domain: String,
    val displayName: String? = null,
)

/**
 * Builds the REGISTER requests for one account over a WebSocket, shaped like
 * sip.js's (RFC 7118): the phone has no reachable address of its own, so Via
 * and Contact name a random `.invalid` host and `transport=ws`, and the
 * switch sends everything back over the same WebSocket.
 *
 * Pure: it builds and answers messages, it does not send them.
 */
class Registration(
    val account: SipAccount,
    /** Seconds the registration lasts; the phone renews it before then. */
    val expires: Int = 600,
    private val random: Random = Random.Default,
    val userAgent: String = "VoipAppz Phone",
) {
    /** The made-up host for Via and Contact (RFC 7118 section 5.2). */
    val viaHost: String = "${token(12)}.invalid"
    private val contactUser = token(8)
    val callId: String = token(22)
    private val fromTag = token(10)
    private var cseq = 0
    private var nonceCount = 0
    private var lastNonce: String? = null

    val contact: String get() = "<sip:$contactUser@$viaHost;transport=ws>"

    private val aor get() = "sip:${account.username}@${account.domain}"
    private val requestUri get() = "sip:${account.domain}"

    /** A new REGISTER: the first one, or a renewal. [expires] 0 unregisters. */
    fun register(expires: Int = this.expires): SipRequest {
        cseq++
        val name = account.displayName?.let { "\"${it.replace("\"", "")}\" " } ?: ""
        return SipRequest(
            "REGISTER", requestUri,
            listOf(
                "Via" to "SIP/2.0/WSS $viaHost;branch=${newBranch()}",
                "Max-Forwards" to "70",
                "To" to "<$aor>",
                "From" to "$name<$aor>;tag=$fromTag",
                "Call-ID" to callId,
                "CSeq" to "$cseq REGISTER",
                "Contact" to "$contact;expires=$expires",
                "Expires" to "$expires",
                "Allow" to "INVITE,ACK,CANCEL,BYE,OPTIONS,INFO,NOTIFY,REFER",
                "Supported" to "path,outbound",
                "User-Agent" to userAgent,
            ),
        )
    }

    /**
     * The REGISTER to send after a 401/407 for [request]: the same request
     * with a new CSeq and branch and the digest answer. Null when the
     * response carries no Digest challenge we can answer, or when the switch
     * rejects the same nonce twice (a wrong password), so the phone does not
     * loop.
     */
    fun authenticate(request: SipRequest, response: SipResponse): SipRequest? {
        val proxy = response.status == 407
        if (response.status != 401 && !proxy) return null
        val challenge = response.header(if (proxy) "Proxy-Authenticate" else "WWW-Authenticate")
            ?.let(DigestChallenge::parse) ?: return null
        val alreadyAnswered = request.header(if (proxy) "Proxy-Authorization" else "Authorization") != null
        if (alreadyAnswered && challenge.nonce == lastNonce && !challenge.stale) return null

        nonceCount = if (challenge.nonce == lastNonce) nonceCount + 1 else 1
        lastNonce = challenge.nonce
        cseq++
        val answer = Digest.authorization(challenge, request.method, request.uri,
            account.username, account.password, cnonce = token(16), nc = nonceCount)
        return request.with(
            "Via" to "SIP/2.0/WSS $viaHost;branch=${newBranch()}",
            "CSeq" to "$cseq ${request.method}",
            (if (proxy) "Proxy-Authorization" else "Authorization") to answer,
        )
    }

    /** Seconds the switch actually granted, from a 200 OK (its Contact expires, or Expires). */
    fun grantedExpires(ok: SipResponse): Int {
        val fromContact = ok.headers("Contact").firstNotNullOfOrNull { value ->
            if (!value.contains(viaHost)) null
            else Regex("expires=(\\d+)", RegexOption.IGNORE_CASE).find(value)?.groupValues?.get(1)?.toIntOrNull()
        }
        return fromContact ?: ok.header("Expires")?.trim()?.toIntOrNull() ?: expires
    }

    private fun newBranch() = "z9hG4bK" + token(16)

    private fun token(length: Int): String {
        val chars = "abcdefghijklmnopqrstuvwxyz0123456789"
        return (1..length).map { chars[random.nextInt(chars.length)] }.joinToString("")
    }
}
