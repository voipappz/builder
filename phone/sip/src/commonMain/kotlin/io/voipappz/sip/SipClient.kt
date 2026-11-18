package io.voipappz.sip

import io.ktor.client.HttpClient
import io.ktor.client.plugins.websocket.WebSockets
import io.ktor.client.plugins.websocket.webSocketSession
import io.ktor.client.request.header
import io.ktor.websocket.DefaultWebSocketSession
import io.ktor.websocket.Frame
import io.ktor.websocket.close
import io.ktor.websocket.readBytes
import io.ktor.websocket.readText
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.selects.select
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeout

/** Where SIP messages go and come from: one WebSocket to the switch. */
interface SipTransport {
    /** Messages as they arrive. Only collectors present at the time see a message. */
    val incoming: SharedFlow<SipMessage>
    /** Completes when the connection ends: normally, or with the error that ended it. */
    val closed: Deferred<Unit>
    suspend fun send(message: SipMessage)
    suspend fun close()
}

class TransportClosedException(cause: Throwable? = null) : Exception("the connection to the switch closed", cause)

/**
 * SIP over a WebSocket (RFC 7118): subprotocol `sip`, one message per frame.
 * Text frames are the norm; binary frames carrying UTF-8 SIP are accepted too
 * (RFC 7118 allows both). The reader never fails the caller's scope: when the
 * socket ends or breaks, [closed] completes and waiting requests fail.
 */
class WebSocketTransport private constructor(
    private val session: DefaultWebSocketSession,
    scope: CoroutineScope,
) : SipTransport {
    private val messages = MutableSharedFlow<SipMessage>(extraBufferCapacity = 64)
    override val incoming: SharedFlow<SipMessage> = messages
    private val ended = CompletableDeferred<Unit>()
    override val closed: Deferred<Unit> = ended

    private val reader: Job = scope.launch {
        try {
            for (frame in session.incoming) {
                val text = when (frame) {
                    is Frame.Text -> frame.readText()
                    is Frame.Binary -> frame.readBytes().decodeToString()
                    else -> continue
                }
                if (text.isBlank()) continue // CRLF keep-alives some switches send
                runCatching { SipMessage.parse(text) }.onSuccess { messages.emit(it) }
            }
            ended.complete(Unit)
        } catch (e: CancellationException) {
            ended.complete(Unit)
            throw e
        } catch (e: Exception) {
            ended.completeExceptionally(TransportClosedException(e))
        }
    }

    override suspend fun send(message: SipMessage) {
        if (ended.isCompleted) throw TransportClosedException()
        session.send(Frame.Text(message.serialize()))
    }

    override suspend fun close() {
        reader.cancel()
        runCatching { session.close() }
        ended.complete(Unit)
    }

    companion object {
        fun httpClient(): HttpClient = HttpClient { install(WebSockets) }

        /** Opens the WebSocket. Only `wss://` (encrypted) is accepted: the switch sees the password digest. */
        suspend fun connect(client: HttpClient, url: String, scope: CoroutineScope): WebSocketTransport {
            require(url.startsWith("wss://")) { "use wss:// (encrypted), not $url" }
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

/** Sends requests and waits for their final answers. One REGISTER at a time. */
class SipClient(private val transport: SipTransport, private val timeoutMillis: Long = 10_000) {
    private val registering = Mutex()

    /**
     * Registers (or with [expires] 0 unregisters), answering the digest
     * challenge and a 423 "interval too brief" once each. The whole exchange
     * has one deadline, so a switch that keeps challenging cannot hold it open.
     */
    suspend fun register(registration: Registration, expires: Int = registration.expires): RegisterResult =
        registering.withLock {
            withTimeout(timeoutMillis * 3) {
                var asked = expires
                var request = registration.register(asked)
                var retriedInterval = false
                while (true) {
                    val response = exchange(registration, request)
                    when {
                        response.status in 200..299 ->
                            return@withTimeout RegisterResult.Registered(if (asked == 0) 0 else registration.grantedExpires(response))
                        response.status == 401 || response.status == 407 ->
                            request = registration.authenticate(request, response)
                                ?: return@withTimeout RegisterResult.Refused(response.status, response.reason)
                        response.status == 423 && !retriedInterval && asked > 0 -> {
                            val min = response.header("Min-Expires")?.trim()?.toIntOrNull()
                                ?: return@withTimeout RegisterResult.Refused(423, response.reason)
                            retriedInterval = true
                            asked = maxOf(min, asked)
                            request = registration.register(asked)
                        }
                        else -> return@withTimeout RegisterResult.Refused(response.status, response.reason)
                    }
                }
                @Suppress("UNREACHABLE_CODE") error("unreachable")
            }
        }

    /**
     * Sends one REGISTER without credentials and returns the switch's first
     * final answer (normally a 401 challenge), without answering it: a
     * connection check that makes no login attempt.
     */
    suspend fun probe(registration: Registration): SipResponse =
        registering.withLock { exchange(registration, registration.register()) }

    /**
     * One client transaction: the final response whose top Via branch,
     * Call-ID and CSeq match the request (1xx skipped). Fails at once if the
     * connection closes.
     */
    private suspend fun exchange(registration: Registration, request: SipRequest): SipResponse = withTimeout(timeoutMillis) {
        coroutineScope {
            val branch = registration.branchOf(request)
            val callId = request.callId
            val cseq = request.cseq
            // Undispatched: the wait is listening before the request goes out,
            // so a fast answer is not missed.
            val answer = async(start = CoroutineStart.UNDISPATCHED) {
                transport.incoming.first {
                    it is SipResponse && it.status >= 200 && it.callId == callId && it.cseq == cseq &&
                        registration.branchOf(it) == branch
                } as SipResponse
            }
            transport.send(request)
            select {
                answer.onAwait { it }
                transport.closed.onAwait {
                    answer.cancel()
                    throw TransportClosedException()
                }
            }
        }
    }
}
