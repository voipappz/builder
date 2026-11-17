package io.voipappz.sip

import io.ktor.client.HttpClient
import io.ktor.client.plugins.websocket.WebSockets
import io.ktor.client.plugins.websocket.webSocketSession
import io.ktor.client.request.header
import io.ktor.websocket.DefaultWebSocketSession
import io.ktor.websocket.Frame
import io.ktor.websocket.close
import io.ktor.websocket.readText
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout

/** Where SIP messages go and come from: one WebSocket to the switch. */
interface SipTransport {
    val incoming: SharedFlow<SipMessage>
    suspend fun send(message: SipMessage)
    suspend fun close()
}

/**
 * SIP over a WebSocket (RFC 7118): subprotocol `sip`, one message per text
 * frame. [client] must have the [WebSockets] plugin installed.
 */
class WebSocketTransport private constructor(
    private val session: DefaultWebSocketSession,
    scope: CoroutineScope,
) : SipTransport {
    private val messages = MutableSharedFlow<SipMessage>(extraBufferCapacity = 64)
    override val incoming: SharedFlow<SipMessage> = messages
    private val reader: Job = scope.launch {
        for (frame in session.incoming) {
            if (frame !is Frame.Text) continue
            val text = frame.readText()
            if (text.isBlank()) continue // keep-alive (RFC 7118 has none, some switches send CRLF)
            runCatching { SipMessage.parse(text) }.onSuccess { messages.emit(it) }
        }
    }

    override suspend fun send(message: SipMessage) {
        session.send(Frame.Text(message.serialize()))
    }

    override suspend fun close() {
        reader.cancel()
        session.close()
    }

    companion object {
        fun httpClient(): HttpClient = HttpClient { install(WebSockets) }

        suspend fun connect(client: HttpClient, url: String, scope: CoroutineScope): WebSocketTransport {
            val session = client.webSocketSession(url) { header("Sec-WebSocket-Protocol", "sip") }
            return WebSocketTransport(session, scope)
        }
    }
}

/** What the switch said to a REGISTER. */
sealed class RegisterResult {
    data class Registered(val expires: Int) : RegisterResult()
    data class Refused(val status: Int, val reason: String) : RegisterResult()
}

/** Sends a request and waits for its final answer, answering one digest challenge on the way. */
class SipClient(private val transport: SipTransport, private val timeoutMillis: Long = 10_000) {

    suspend fun register(registration: Registration, expires: Int = registration.expires): RegisterResult {
        var request = registration.register(expires)
        while (true) {
            val response = exchange(request)
            when {
                response.status in 200..299 ->
                    return RegisterResult.Registered(if (expires == 0) 0 else registration.grantedExpires(response))
                response.status == 401 || response.status == 407 ->
                    request = registration.authenticate(request, response)
                        ?: return RegisterResult.Refused(response.status, response.reason)
                else -> return RegisterResult.Refused(response.status, response.reason)
            }
        }
    }

    /** One transaction: the final response with this request's Call-ID and CSeq (1xx skipped). */
    private suspend fun exchange(request: SipRequest): SipResponse = withTimeout(timeoutMillis) {
        coroutineScope {
            val callId = request.callId
            val cseq = request.cseq
            // Undispatched: the wait is listening before the request goes out,
            // so a fast answer is not missed.
            val answer = async(start = CoroutineStart.UNDISPATCHED) {
                transport.incoming.first { it is SipResponse && it.status >= 200 && it.callId == callId && it.cseq == cseq }
                    as SipResponse
            }
            transport.send(request)
            answer.await()
        }
    }
}
