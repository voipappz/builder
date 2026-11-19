/* global AudioWorkletProcessor, registerProcessor */
// Runs on the audio thread: collects the microphone (the context runs at
// 16 kHz) into 20 ms packets of 320 samples for the bot call. Loaded from the
// app's dev server, which Builder's page policy allows; a blob: URL is not.
const PACKET_SAMPLES = 320;

class Capture extends AudioWorkletProcessor {
  constructor() {
    super();
    this.buf = new Float32Array(PACKET_SAMPLES);
    this.n = 0;
  }

  process(inputs) {
    const channel = inputs[0] && inputs[0][0];
    if (channel) {
      for (let i = 0; i < channel.length; i += 1) {
        this.buf[this.n] = channel[i];
        this.n += 1;
        if (this.n === PACKET_SAMPLES) {
          this.port.postMessage(this.buf.slice());
          this.n = 0;
        }
      }
    }
    return true;
  }
}

registerProcessor('builder-capture', Capture);
