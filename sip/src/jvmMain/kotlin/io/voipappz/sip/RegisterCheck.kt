package io.voipappz.sip

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.runBlocking
import kotlin.system.exitProcess

/**
 * Registers one account with a real switch over WSS and unregisters again.
 * The account comes only from the environment, and the password is never
 * printed: SIP_USER, SIP_PASSWORD, SIP_DOMAIN (default: the WSS host), SIP_WSS.
 */
fun main() {
    val env = System.getenv()
    val wss = env["SIP_WSS"] ?: fail("set SIP_WSS, e.g. wss://switch.example.com:8443")
    val user = env["SIP_USER"] ?: fail("set SIP_USER")
    val password = env["SIP_PASSWORD"] ?: fail("set SIP_PASSWORD")
    val domain = env["SIP_DOMAIN"] ?: java.net.URI(wss).host
    val account = SipAccount(user, password, domain)

    val ok = runBlocking {
        coroutineScope {
            val http = WebSocketTransport.httpClient()
            var transport: WebSocketTransport? = null
            try {
                transport = WebSocketTransport.connect(http, wss, this)
                println("✓ WebSocket open: $wss (subprotocol sip)")
                val client = SipClient(transport)
                // The browser phone asks for 300 s; the same here.
                val registration = Registration(account, expires = 300)
                val result = try {
                    client.register(registration)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    RegisterResult.Refused(0, "no answer: ${e.message}")
                }
                when (result) {
                    is RegisterResult.Registered -> println("✓ registered $user@$domain for ${result.expires}s")
                    is RegisterResult.Refused -> println("✕ refused: ${result.status} ${result.reason}")
                }
                if (result is RegisterResult.Registered) {
                    val off = runCatching { client.register(registration, expires = 0) }.getOrNull()
                    println(if (off is RegisterResult.Registered) "✓ unregistered" else "! unregister got $off")
                }
                result is RegisterResult.Registered
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                println("✕ could not open $wss: ${e.message}")
                false
            } finally {
                transport?.close()
                http.close()
            }
        }
    }
    exitProcess(if (ok) 0 else 1)
}

private fun fail(message: String): Nothing {
    System.err.println(message)
    exitProcess(2)
}
