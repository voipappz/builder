import { describe, expect, it } from 'vitest';
import { fromPcm16, MAX_UTTERANCE_SAMPLES, PACKET_SAMPLES, SAMPLE_RATE, toPcm16 } from './botCall';

describe('bot call audio', () => {
  it('uses Earshot native framing: 20 ms packets at 16 kHz', () => {
    expect(SAMPLE_RATE).toBe(16000);
    expect(PACKET_SAMPLES / SAMPLE_RATE).toBe(0.02);
    // The bot drops a call whose utterance passes 20 s.
    expect(MAX_UTTERANCE_SAMPLES).toBeLessThan(SAMPLE_RATE * 20);
  });

  it('turns float samples into PCM16 and back, clipping out-of-range values', () => {
    const pcm = toPcm16(new Float32Array([0, 1, -1, 0.5, 2, -2]));
    expect(Array.from(pcm)).toEqual([0, 32767, -32768, 16383, 32767, -32768]);
    const back = fromPcm16(pcm.buffer);
    expect(back[1]).toBeCloseTo(1, 3);
    expect(back[2]).toBe(-1);
    expect(back[3]).toBeCloseTo(0.5, 3);
  });
});
