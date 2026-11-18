package io.voipappz.sip

import okio.ByteString.Companion.encodeUtf8

/**
 * HTTP Digest, as SIP uses it (RFC 3261 section 22, RFC 2617): the switch
 * answers a request with 401 (or 407) and a challenge; the phone repeats the
 * request with a response computed from its password. The password itself is
 * never sent.
 */
data class DigestChallenge(
    val realm: String,
    val nonce: String,
    val opaque: String? = null,
    val algorithm: String = "MD5",
    /** The qop values offered, e.g. ["auth"]; empty for the old RFC 2069 form. */
    val qop: List<String> = emptyList(),
    val stale: Boolean = false,
) {
    companion object {
        /** Reads a `WWW-Authenticate` / `Proxy-Authenticate` value; null unless it is Digest. */
        fun parse(value: String): DigestChallenge? {
            val trimmed = value.trim()
            if (!trimmed.startsWith("Digest", ignoreCase = true)) return null
            val params = parseParams(trimmed.substring("Digest".length))
            return DigestChallenge(
                realm = params["realm"] ?: return null,
                nonce = params["nonce"] ?: return null,
                opaque = params["opaque"],
                algorithm = params["algorithm"] ?: "MD5",
                qop = params["qop"]?.split(',')?.map { it.trim() }?.filter { it.isNotEmpty() } ?: emptyList(),
                stale = params["stale"].equals("true", ignoreCase = true),
            )
        }

        /** `a="x, y", b=z` as a map; quoted values may hold commas. */
        internal fun parseParams(text: String): Map<String, String> {
            val result = mutableMapOf<String, String>()
            var i = 0
            while (i < text.length) {
                while (i < text.length && (text[i] == ',' || text[i].isWhitespace())) i++
                val eq = text.indexOf('=', i).takeIf { it >= 0 } ?: break
                val key = text.substring(i, eq).trim().lowercase()
                i = eq + 1
                while (i < text.length && text[i].isWhitespace()) i++
                val value: String
                if (i < text.length && text[i] == '"') {
                    val sb = StringBuilder()
                    i++
                    while (i < text.length && text[i] != '"') {
                        if (text[i] == '\\' && i + 1 < text.length) i++
                        sb.append(text[i]); i++
                    }
                    i++ // closing quote
                    value = sb.toString()
                } else {
                    val end = text.indexOf(',', i).takeIf { it >= 0 } ?: text.length
                    value = text.substring(i, end).trim()
                    i = end
                }
                result[key] = value
            }
            return result
        }
    }
}

object Digest {
    /**
     * The `Authorization` (or `Proxy-Authorization`) value for one request.
     * [nc] counts uses of the same nonce, starting at 1; [cnonce] is a fresh
     * random string per request.
     */
    fun authorization(
        challenge: DigestChallenge,
        method: String,
        uri: String,
        username: String,
        password: String,
        cnonce: String,
        nc: Int = 1,
    ): String {
        if (!challenge.algorithm.equals("MD5", ignoreCase = true)) {
            throw UnsupportedOperationException("digest algorithm ${challenge.algorithm} is not supported")
        }
        val ha1 = md5("$username:${challenge.realm}:$password")
        val ha2 = md5("$method:$uri")
        val useQop = challenge.qop.any { it.equals("auth", ignoreCase = true) }
        if (challenge.qop.isNotEmpty() && !useQop) {
            throw UnsupportedOperationException("digest qop ${challenge.qop} is not supported (only auth)")
        }
        val ncText = nc.toString(16).padStart(8, '0')
        val response = if (useQop) md5("$ha1:${challenge.nonce}:$ncText:$cnonce:auth:$ha2")
            else md5("$ha1:${challenge.nonce}:$ha2")
        return buildString {
            append("Digest username=\"").append(quote(username)).append('"')
            append(", realm=\"").append(quote(challenge.realm)).append('"')
            append(", nonce=\"").append(quote(challenge.nonce)).append('"')
            append(", uri=\"").append(quote(uri)).append('"')
            append(", response=\"").append(response).append('"')
            append(", algorithm=MD5")
            if (useQop) append(", qop=auth, nc=").append(ncText).append(", cnonce=\"").append(quote(cnonce)).append('"')
            challenge.opaque?.let { append(", opaque=\"").append(quote(it)).append('"') }
        }
    }

    internal fun md5(text: String): String = text.encodeUtf8().md5().hex()

    private fun quote(text: String) = text.replace("\\", "\\\\").replace("\"", "\\\"")
}
