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
    // SIP_PROBE=1: one REGISTER without credentials; print the switch's answer
    // (normally its 401 challenge) and stop. No password, no login attempt.
    val probe = env["SIP_PROBE"] == "1"
    val user = env["SIP_USER"] ?: if (probe) "probe" else fail("set SIP_USER")
    val password = env["SIP_PASSWORD"] ?: if (probe) "" else fail("set SIP_PASSWORD")
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
                if (probe) {
                    val answer = client.probe(Registration(account, expires = 300))
                    println("✓ the switch answered: ${answer.status} ${answer.reason}")
                    answer.header("WWW-Authenticate")?.let(DigestChallenge::parse)?.let {
                        println("  challenge: realm=${it.realm}, algorithm=${it.algorithm}, qop=${it.qop.ifEmpty { listOf("none") }}")
                    }
                    answer.header("User-Agent")?.let { println("  server: $it") }
                    return@coroutineScope answer.status == 401 || answer.status == 407
                }
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
                // Success means registered and unregistered: a check that
                // leaves a registration behind has not cleaned up.
                val unregistered = result is RegisterResult.Registered && run {
                    val off = runCatching { client.register(registration, expires = 0) }.getOrNull()
                    println(if (off is RegisterResult.Registered) "✓ unregistered" else "✕ unregister failed: $off")
                    off is RegisterResult.Registered
                }
                unregistered
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
