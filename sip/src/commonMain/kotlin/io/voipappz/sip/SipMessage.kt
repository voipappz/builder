package io.voipappz.sip

/**
 * A SIP message as it travels over the WebSocket (RFC 7118): one text frame,
 * a start line, headers and an optional body.
 *
 * Headers keep their order and may repeat (several Via lines). Lookups ignore
 * case and understand the compact forms switches sometimes send (`v:` for
 * Via, `i:` for Call-ID, ...).
 */
sealed class SipMessage(
    val headers: List<Pair<String, String>>,
    val body: String,
) {
    /** The first value of a header, or null. */
    fun header(name: String): String? = headers(name).firstOrNull()

    /** Every value of a header, in order. */
    fun headers(name: String): List<String> {
        val wanted = canonical(name)
        return headers.filter { canonical(it.first) == wanted }.map { it.second }
    }

    val callId: String? get() = header("Call-ID")

    /** The CSeq number and method, e.g. (2, "REGISTER"). */
    val cseq: Pair<Int, String>?
        get() = header("CSeq")?.trim()?.split(Regex("\\s+"))?.takeIf { it.size == 2 }
            ?.let { (n, m) -> n.toIntOrNull()?.let { it to m } }

    protected abstract fun startLine(): String

    /** The text sent in one WebSocket frame. Content-Length is always written from the body. */
    fun serialize(): String = buildString {
        append(startLine()).append(CRLF)
        headers.filterNot { canonical(it.first) == "content-length" }.forEach { (name, value) ->
            append(name).append(": ").append(value).append(CRLF)
        }
        append("Content-Length: ").append(body.encodeToByteArray().size).append(CRLF)
        append(CRLF)
        append(body)
    }

    override fun toString(): String = serialize()

    companion object {
        const val CRLF = "\r\n"

        private val compact = mapOf(
            "v" to "via", "f" to "from", "t" to "to", "i" to "call-id", "m" to "contact",
            "l" to "content-length", "c" to "content-type", "k" to "supported", "e" to "content-encoding",
        )

        internal fun canonical(name: String): String = name.trim().lowercase().let { compact[it] ?: it }

        /** Reads one message; throws [SipParseException] when it is not SIP. */
        fun parse(text: String): SipMessage {
            val split = text.indexOf("\r\n\r\n").takeIf { it >= 0 }
                ?: text.indexOf("\n\n").takeIf { it >= 0 }
                ?: text.length
            val head = text.substring(0, split)
            val body = text.substring(split).removePrefix("\r\n\r\n").removePrefix("\n\n")
            val lines = head.split("\r\n", "\n").filter { it.isNotEmpty() }
            if (lines.isEmpty()) throw SipParseException("empty message")

            // Folded header lines (a leading space) continue the previous one.
            val headers = mutableListOf<Pair<String, String>>()
            for (line in lines.drop(1)) {
                if (line.first().isWhitespace() && headers.isNotEmpty()) {
                    val (name, value) = headers.removeAt(headers.lastIndex)
                    headers.add(name to "$value ${line.trim()}")
                    continue
                }
                val colon = line.indexOf(':')
                if (colon <= 0) throw SipParseException("bad header line: $line")
                headers.add(line.substring(0, colon).trim() to line.substring(colon + 1).trim())
            }

            val declared = headers.firstOrNull { canonical(it.first) == "content-length" }?.second?.trim()?.toIntOrNull()
            val bytes = body.encodeToByteArray()
            if (declared != null && declared > bytes.size) throw SipParseException("body shorter than Content-Length")
            val exactBody = if (declared != null) bytes.copyOf(declared).decodeToString() else body

            val start = lines.first()
            return if (start.startsWith("SIP/2.0 ")) {
                val parts = start.split(" ", limit = 3)
                val status = parts.getOrNull(1)?.toIntOrNull() ?: throw SipParseException("bad status line: $start")
                SipResponse(status, parts.getOrElse(2) { "" }, headers, exactBody)
            } else {
                val parts = start.split(" ")
                if (parts.size != 3 || parts[2] != "SIP/2.0") throw SipParseException("bad request line: $start")
                SipRequest(parts[0], parts[1], headers, exactBody)
            }
        }
    }
}

class SipRequest(
    val method: String,
    val uri: String,
    headers: List<Pair<String, String>>,
    body: String = "",
) : SipMessage(headers, body) {
    override fun startLine() = "$method $uri SIP/2.0"

    /** The same request with headers replaced or added (by name), e.g. a new CSeq and Authorization. */
    fun with(vararg changes: Pair<String, String>): SipRequest {
        val names = changes.map { canonical(it.first) }.toSet()
        return SipRequest(method, uri, headers.filterNot { canonical(it.first) in names } + changes, body)
    }
}

class SipResponse(
    val status: Int,
    val reason: String,
    headers: List<Pair<String, String>>,
    body: String = "",
) : SipMessage(headers, body) {
    override fun startLine() = "SIP/2.0 $status $reason"
}

class SipParseException(message: String) : Exception(message)
