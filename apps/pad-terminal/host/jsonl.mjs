// Strict JSONL framing shared by the host transport and Pi RPC children.
// Records split on LF only; U+2028/U+2029 inside JSON strings are not boundaries.
export const MAX_FRAME_BYTES = 8 * 1024 * 1024;

/**
 * Incremental LF-only frame decoder.
 * `onFrame(text)` receives one complete record without its trailing LF/CR.
 * `onOversize(bytes)` fires once when a single frame exceeds `maxBytes`; the
 * oversized buffer is dropped so later frames can still be decoded.
 */
export function createFrameDecoder({ maxBytes = MAX_FRAME_BYTES, onFrame, onOversize } = {}) {
  let buffer = Buffer.alloc(0);
  return {
    feed(chunk) {
      const incoming = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      buffer = buffer.length === 0 ? incoming : Buffer.concat([buffer, incoming]);
      let newline = buffer.indexOf(0x0a);
      while (newline >= 0) {
        let frame = buffer.subarray(0, newline);
        buffer = buffer.subarray(newline + 1);
        if (frame.length > 0 && frame[frame.length - 1] === 0x0d) frame = frame.subarray(0, frame.length - 1);
        if (frame.length > 0 && typeof onFrame === 'function') onFrame(frame.toString('utf8'));
        newline = buffer.indexOf(0x0a);
      }
      if (buffer.length > maxBytes) {
        const bytes = buffer.length;
        buffer = Buffer.alloc(0);
        if (typeof onOversize === 'function') onOversize(bytes);
        return false;
      }
      return true;
    },
    get bufferedBytes() {
      return buffer.length;
    },
    reset() {
      buffer = Buffer.alloc(0);
    },
  };
}

/** Serialize one protocol frame, rejecting anything above the frame budget. */
export function encodeFrame(value, { maxBytes = MAX_FRAME_BYTES } = {}) {
  const line = `${JSON.stringify(value)}\n`;
  const bytes = Buffer.byteLength(line, 'utf8');
  if (bytes > maxBytes) throw new Error(`Frame exceeds the ${maxBytes}-byte budget`);
  return line;
}

/** Write one protocol frame. Returns false when the frame could not be written. */
export function writeFrame(stream, value, options) {
  const line = encodeFrame(value, options);
  return stream.write(line);
}

export function parseJson(text) {
  try {
    return { ok: true, value: JSON.parse(text) };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error : new Error(String(error)) };
  }
}

export function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
