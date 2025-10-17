/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { Chat } from '../components/types';
import { Dispatch, SetStateAction } from 'react';

export const setChatsWithFiles = (
  updater: ((prev: Chat[]) => Chat[]) | Chat[],
  setChats: Dispatch<SetStateAction<Chat[]>>,
  receivedFilesRef: React.RefObject<{
    [file_id: string]: {
      chunks: string[];
      name: string;
      size: number;
      received: number;
      total: number;
      blob?: Blob;
      assembled?: boolean;
      fileName?: string;
    };
  }>,
  setFileAssemblyTick: Dispatch<SetStateAction<number>>
) => {
  setChats((prev: Chat[]) => {
    const next = typeof updater === 'function' ? updater(prev) : updater;
    let fileAssembled = false;
    // Scan for file transfer events in all messages
    for (const chat of next) {
      for (const m of chat.messages) {
        const isFileStart =
          typeof m === 'object' &&
          'type' in m &&
          m.type === 'FILE_START' &&
          'payload' in m &&
          m.payload &&
          typeof m.payload === 'object' &&
          'file_id' in m.payload;
        const isFileChunk =
          typeof m === 'object' &&
          'type' in m &&
          m.type === 'FILE_CHUNK' &&
          'payload' in m &&
          m.payload &&
          typeof m.payload === 'object' &&
          'file_id' in m.payload &&
          'ciphertext' in m.payload;
        const isFileEnd =
          typeof m === 'object' &&
          'type' in m &&
          m.type === 'FILE_END' &&
          'payload' in m &&
          m.payload &&
          typeof m.payload === 'object' &&
          'file_id' in m.payload;
        // FILE_START: initialise file meta
        if (
          isFileStart ||
          (typeof m === 'object' &&
            'message' in m &&
            typeof m.message === 'string' &&
            m.message.startsWith('📎 Received file (ID: '))
        ) {
          let fileId: string | undefined = undefined;
          let name: string | undefined = undefined;
          let size: number | undefined = undefined;
          if (isFileStart) {
            const payload = m.payload as { file_id: string; name?: string; size?: number };
            fileId = payload.file_id;
            name = payload.name;
            size = payload.size;
          }
          if (!fileId && 'message' in m && typeof m.message === 'string') {
            const match = m.message.match(/ID: ([a-f0-9-]+)/);
            if (match) fileId = match[1];
          }
          if (fileId && !receivedFilesRef.current[fileId]) {
            // Try to get name from message
            if (!name && 'message' in m && typeof m.message === 'string') {
              const nameMatch = m.message.match(/file: ([^)]+)/);
              if (nameMatch) name = nameMatch[1];
            }
            receivedFilesRef.current[fileId] = {
              chunks: [],
              name: name || 'attachment.bin',
              size: size || 0,
              received: 0,
              total: 0,
              blob: undefined,
              assembled: false,
              fileName: name || 'attachment.bin',
            };
          }
        }
        // FILE_CHUNK: add chunk to file meta
        if (isFileChunk) {
          const payload = m.payload as { file_id: string; index: number; ciphertext: string };
          const fileId = payload.file_id;
          if (!receivedFilesRef.current[fileId]) {
            receivedFilesRef.current[fileId] = {
              chunks: [],
              name: 'attachment.bin',
              size: 0,
              received: 0,
              total: 0,
              blob: undefined,
              assembled: false,
              fileName: 'attachment.bin',
            };
          }
          const fileMeta = receivedFilesRef.current[fileId];
          // Insert chunk at correct index if not already present
          if (typeof fileMeta.chunks[payload.index] === 'undefined') {
            fileMeta.chunks[payload.index] = payload.ciphertext;
            fileMeta.received++;
          }
        }
        // FILE_END: set total and try to assemble
        if (isFileEnd) {
          const payload = m.payload as { file_id: string };
          const fileId = payload.file_id;
          const fileMeta = receivedFilesRef.current[fileId];
          if (fileMeta) {
            // The total should be the highest index + 1
            const chunkCount = fileMeta.chunks.filter((x) => typeof x !== 'undefined').length;
            fileMeta.total = chunkCount;
            // Only assemble if all chunks are present and not undefined
            if (
              fileMeta.received === fileMeta.total &&
              fileMeta.chunks.length === fileMeta.total &&
              fileMeta.chunks.every((c) => typeof c === 'string') &&
              !fileMeta.blob
            ) {
              // Sort chunks by index to ensure correct order
              const sortedChunks = fileMeta.chunks.map((b64) =>
                Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
              );
              const blob = new Blob(sortedChunks, { type: 'application/octet-stream' });
              fileMeta.blob = blob;
              fileMeta.assembled = true;
              fileAssembled = true;
            }
          }
        }
        // Fallback: If message is a display message for file, try to assemble if all chunks present
        if (
          'message' in m &&
          typeof m.message === 'string' &&
          m.message.startsWith('📎 Received file (ID: ')
        ) {
          const match = m.message.match(/ID: ([a-f0-9-]+)/);
          if (match) {
            const fileId = match[1];
            const fileMeta = receivedFilesRef.current[fileId];
            if (
              fileMeta &&
              fileMeta.received === fileMeta.total &&
              fileMeta.chunks.length === fileMeta.total &&
              fileMeta.chunks.every((c) => typeof c === 'string') &&
              !fileMeta.blob
            ) {
              // Sort chunks by index to ensure correct order
              const sortedChunks = fileMeta.chunks.map((b64) =>
                Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))
              );
              const blob = new Blob(sortedChunks, { type: 'application/octet-stream' });
              fileMeta.blob = blob;
              fileMeta.assembled = true;
              fileAssembled = true;
            }
          }
        }
      }
    }
    // If any file was assembled, force a re-render
    if (fileAssembled) setFileAssemblyTick((tick) => tick + 1);
    return next;
  });
};
