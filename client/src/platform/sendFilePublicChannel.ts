/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { User } from './db';

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  return btoa(String.fromCharCode(...new Uint8Array(buffer)));
}

async function hashFileSHA256(file: File): Promise<string> {
  const arrayBuffer = await file.arrayBuffer();
  const hashBuffer = await window.crypto.subtle.digest('SHA-256', arrayBuffer);
  return Array.from(new Uint8Array(hashBuffer))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

export async function sendFilePublicChannel(
  file: File,
  user: User,
  sentFilesRef: React.RefObject<{ [file_id: string]: Blob }>,
  channelId: string
) {
  const file_id = crypto.randomUUID();
  const sha256 = await hashFileSHA256(file);
  const now = Date.now();
  const manifest = {
    type: 'FILE_START',
    from: user.user_id,
    to: channelId,
    ts: [Math.floor(now / 1000), (now % 1000) * 1_000_000],
    payload: {
      file_id,
      name: file.name,
      size: file.size,
      sha256,
      mode: 'public',
    },
    sig: '',
  };
  await fetch('http://localhost:3000/api/public_channel/file_start', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(manifest),
  });

  const chunkSize = 64 * 1024;
  let index = 0;
  const chunks: string[] = [];
  for (let offset = 0; offset < file.size; offset += chunkSize) {
    const chunk = file.slice(offset, offset + chunkSize);
    const arrayBuffer = await chunk.arrayBuffer();
    const ciphertext = arrayBufferToBase64(arrayBuffer);
    chunks.push(ciphertext);
    const chunkNow = Date.now();
    const chunkMsg = {
      type: 'FILE_CHUNK',
      from: user.user_id,
      to: channelId,
      ts: [Math.floor(chunkNow / 1000), (chunkNow % 1000) * 1_000_000],
      payload: {
        file_id,
        index,
        ciphertext,
      },
      sig: '',
    };
    await fetch('http://localhost:3000/api/public_channel/file_chunk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(chunkMsg),
    });
    index++;
  }
  // Store the sent file blob for download (sender side)
  if (sentFilesRef && sentFilesRef.current) {
    sentFilesRef.current[file_id] = new Blob(
      chunks.map((b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))),
      { type: 'application/octet-stream' }
    );
  }

  const endNow = Date.now();
  const endMsg = {
    type: 'FILE_END',
    from: user.user_id,
    to: channelId,
    ts: [Math.floor(endNow / 1000), (endNow % 1000) * 1_000_000],
    payload: { file_id },
    sig: '',
  };
  await fetch('http://localhost:3000/api/public_channel/file_end', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(endMsg),
  });
}
