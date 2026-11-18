package io.voipappz.phone

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import io.voipappz.sip.RegisterResult
import io.voipappz.sip.Registration
import io.voipappz.sip.SipAccount
import io.voipappz.sip.SipClient
import io.voipappz.sip.WebSocketTransport
import kotlinx.coroutines.launch

/** The switch every phone talks to, the same as the browser phone's. */
const val DEFAULT_WSS = "wss://switch.voipappz.io:8443"

/**
 * The first screen, shared by Android and iPhone: sign in with an extension
 * and see whether the switch accepts it. Calls come next. The password stays
 * in memory only, for this test.
 */
@Composable
fun App() {
    MaterialTheme {
        Surface(Modifier.fillMaxSize()) {
            var server by remember { mutableStateOf(DEFAULT_WSS) }
            var extension by remember { mutableStateOf("") }
            var password by remember { mutableStateOf("") }
            var status by remember { mutableStateOf("Not registered") }
            var busy by remember { mutableStateOf(false) }
            val scope = rememberCoroutineScope()

            Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("VoipAppz Phone", style = MaterialTheme.typography.headlineSmall)
                OutlinedTextField(server, { server = it }, label = { Text("Server") }, singleLine = true,
                    modifier = Modifier.fillMaxWidth())
                OutlinedTextField(extension, { extension = it }, label = { Text("Extension") }, singleLine = true,
                    modifier = Modifier.fillMaxWidth())
                OutlinedTextField(password, { password = it }, label = { Text("Password") }, singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    modifier = Modifier.fillMaxWidth())
                Button(
                    enabled = !busy && extension.isNotBlank() && password.isNotEmpty(),
                    onClick = {
                        busy = true
                        status = "Registering…"
                        scope.launch {
                            status = register(server.trim(), extension.trim(), password)
                            busy = false
                        }
                    },
                ) { Text("Register") }
                Text(status)
            }
        }
    }
}

/** One REGISTER and its answer, as a line for the screen. */
private suspend fun register(server: String, extension: String, password: String): String {
    val domain = server.removePrefix("wss://").substringBefore(':').substringBefore('/')
    val http = WebSocketTransport.httpClient()
    return try {
        kotlinx.coroutines.coroutineScope {
            val transport = WebSocketTransport.connect(http, server, this)
            try {
                val client = SipClient(transport)
                val registration = Registration(SipAccount(extension, password, domain), expires = 300)
                when (val result = client.register(registration)) {
                    is RegisterResult.Registered -> {
                        // A test screen: it does not stay registered (renewal
                        // comes later), so it unregisters again at once.
                        val off = runCatching { client.register(registration, expires = 0) }.getOrNull()
                        "Registered $extension@$domain (granted ${result.expires}s), then " +
                            if (off is RegisterResult.Registered) "unregistered." else "unregister failed."
                    }
                    is RegisterResult.Refused -> "Refused: ${result.status} ${result.reason}"
                }
            } finally {
                transport.close()
            }
        }
    } catch (e: kotlinx.coroutines.CancellationException) {
        throw e
    } catch (e: Exception) {
        "Could not reach the switch: ${e.message}"
    } finally {
        http.close()
    }
}
